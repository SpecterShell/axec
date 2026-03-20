use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::Result;

struct OutputBufferInner {
    ring: VecDeque<u8>,
    file: File,
    max_bytes: usize,
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

        Ok(Self {
            path,
            inner: Mutex::new(OutputBufferInner {
                ring: VecDeque::new(),
                file,
                max_bytes,
            }),
        })
    }

    pub fn append(&self, data: &[u8]) -> Result<()> {
        let mut inner = self.inner.lock().expect("output buffer mutex poisoned");
        inner.ring.extend(data.iter().copied());
        while inner.ring.len() > inner.max_bytes {
            inner.ring.pop_front();
        }
        inner.file.write_all(data)?;
        inner.file.flush()?;
        Ok(())
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
}
