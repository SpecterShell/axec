use std::fmt;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

use crate::config;
use crate::error::{AxecError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionBackend {
    #[default]
    Pty,
    Pipe,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Ping,
    Run {
        command: String,
        args: Vec<String>,
        name: Option<String>,
        timeout: Option<u64>,
        stopword: Option<String>,
        terminate: bool,
        backend: SessionBackend,
        cwd: Option<PathBuf>,
        env: Vec<EnvVar>,
    },
    Cat {
        session: Option<String>,
        follow: bool,
        stderr: bool,
    },
    Output {
        session: Option<String>,
    },
    List,
    Input {
        session: Option<String>,
        text: String,
        timeout: Option<u64>,
        stopword: Option<String>,
        terminate: bool,
    },
    Signal {
        session: Option<String>,
        signal: String,
    },
    Kill {
        session: Option<String>,
        all: bool,
    },
    Attach {
        session: String,
    },
    Clean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Pong,
    SessionCreated {
        uuid: Uuid,
        name: Option<String>,
    },
    OutputChunk {
        data: String,
        stream: OutputStream,
    },
    CatOutput {
        data: String,
    },
    OutputData {
        data: String,
    },
    SessionList {
        sessions: Vec<SessionInfo>,
    },
    Finished {
        exit_code: Option<i32>,
        timed_out: bool,
        running: bool,
    },
    Ack {
        message: String,
    },
    Cleaned {
        removed: usize,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    Exited { exit_code: i32 },
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Exited { exit_code } => write!(f, "exited({exit_code})"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub uuid: Uuid,
    pub name: Option<String>,
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub pid: Option<u32>,
    #[serde(default)]
    pub backend: SessionBackend,
    pub started_at: i64,
    pub exited_at: Option<i64>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub uuid: Uuid,
    pub name: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<EnvVar>,
    pub pid: Option<u32>,
    pub process_group: Option<i32>,
    #[serde(default)]
    pub backend: SessionBackend,
    pub started_at: i64,
    pub exited_at: Option<i64>,
    pub status: SessionStatus,
}

pub async fn write_frame<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let payload = serde_json::to_vec(value)?;
    if payload.len() > config::MAX_FRAME_BYTES {
        return Err(AxecError::Protocol(
            "frame exceeds maximum size".to_string(),
        ));
    }

    writer.write_u32(payload.len() as u32).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_frame<R, T>(reader: &mut R) -> Result<Option<T>>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(err.into()),
    }

    let len = u32::from_be_bytes(len_buf) as usize;
    if len > config::MAX_FRAME_BYTES {
        return Err(AxecError::Protocol(
            "frame exceeds maximum size".to_string(),
        ));
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    Ok(Some(serde_json::from_slice(&payload)?))
}

#[cfg(test)]
mod tests {
    use super::{Request, Response, read_frame, write_frame};

    #[tokio::test]
    async fn frame_roundtrip() {
        let (mut left, mut right) = tokio::io::duplex(4096);
        let request = Request::Ping;
        let response = Response::Pong;

        let writer = tokio::spawn(async move {
            write_frame(&mut left, &request).await.unwrap();
            write_frame(&mut left, &response).await.unwrap();
        });

        let read_request: Option<Request> = read_frame(&mut right).await.unwrap();
        let read_response: Option<Response> = read_frame(&mut right).await.unwrap();
        writer.await.unwrap();

        assert!(matches!(read_request, Some(Request::Ping)));
        assert!(matches!(read_response, Some(Response::Pong)));
    }
}
