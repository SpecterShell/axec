use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::Result;

struct OutputBufferInner {
    ring: VecDeque<u8>,
    file: File,
    max_bytes: usize,
    total_bytes: u64,
}

pub struct OutputBuffer {
    path: PathBuf,
    inner: Mutex<OutputBufferInner>,
}

impl OutputBuffer {
    pub fn new(path: PathBuf, max_bytes: usize) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;

        let total_bytes = file.metadata()?.len();

        Ok(Self {
            path,
            inner: Mutex::new(OutputBufferInner {
                ring: VecDeque::new(),
                file,
                max_bytes,
                total_bytes,
            }),
        })
    }

    pub fn append(&self, data: &[u8]) -> Result<u64> {
        let mut inner = self.inner.lock().expect("output buffer mutex poisoned");
        inner.ring.extend(data.iter().copied());
        while inner.ring.len() > inner.max_bytes {
            inner.ring.pop_front();
        }
        inner.file.write_all(data)?;
        inner.file.flush()?;
        inner.total_bytes += data.len() as u64;
        Ok(inner.total_bytes)
    }

    pub fn read_all_string(&self) -> Result<String> {
        let data = fs::read(&self.path)?;
        Ok(String::from_utf8_lossy(&data).to_string())
    }

    pub fn read_recent_bytes(&self) -> Vec<u8> {
        self.inner
            .lock()
            .expect("output buffer mutex poisoned")
            .ring
            .iter()
            .copied()
            .collect()
    }

    pub fn read_string_from(&self, start: u64) -> Result<(String, u64)> {
        let end = {
            let mut inner = self.inner.lock().expect("output buffer mutex poisoned");
            inner.file.flush()?;
            inner.total_bytes
        };

        let start = start.min(end);
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(start))?;
        let mut data = Vec::with_capacity((end - start) as usize);
        file.take(end - start).read_to_end(&mut data)?;
        Ok((String::from_utf8_lossy(&data).to_string(), end))
    }

    #[cfg(test)]
    fn ring_len(&self) -> usize {
        self.inner
            .lock()
            .expect("output buffer mutex poisoned")
            .ring
            .len()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::OutputBuffer;

    #[test]
    fn trims_ring_but_keeps_full_log() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("stdout.log");
        let buffer = OutputBuffer::new(path, 4).unwrap();
        buffer.append(b"abcdef").unwrap();

        assert_eq!(buffer.ring_len(), 4);
        assert_eq!(buffer.read_all_string().unwrap(), "abcdef");
    }

    #[test]
    fn reads_incremental_ranges() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("stdout.log");
        let buffer = OutputBuffer::new(path, 8).unwrap();
        let first_end = buffer.append(b"hello").unwrap();
        let second_end = buffer.append(b" world").unwrap();

        let (first, first_snapshot_end) = buffer.read_string_from(0).unwrap();
        let (second, second_snapshot_end) = buffer.read_string_from(first_end).unwrap();

        assert_eq!(first, "hello world");
        assert_eq!(first_snapshot_end, second_end);
        assert_eq!(second, " world");
        assert_eq!(second_snapshot_end, second_end);
    }
}
