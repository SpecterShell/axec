use std::path::Path;
use std::time::Duration;

use tokio::time::sleep;

use crate::config;
use crate::error::{AxecError, Result};
use crate::paths;
use crate::platform;
use crate::protocol::{Request, Response, read_frame, write_frame};
use crate::transport::{self, Connection};

pub struct DaemonConnection {
    stream: Connection,
}

impl DaemonConnection {
    pub async fn connect() -> Result<Self> {
        paths::ensure_base_dirs()?;
        let socket_path = paths::socket_path()?;

        if let Ok(connection) = Self::try_connect(&socket_path).await {
            return Ok(connection);
        }

        platform::spawn_daemon()?;

        for _ in 0..config::CONNECT_RETRIES {
            if let Ok(connection) = Self::try_connect(&socket_path).await {
                return Ok(connection);
            }
            sleep(Duration::from_millis(config::CONNECT_RETRY_DELAY_MS)).await;
        }

        Err(AxecError::Protocol(
            "unable to connect to the axec daemon".to_string(),
        ))
    }

    async fn try_connect(path: &Path) -> Result<Self> {
        Ok(Self {
            stream: transport::connect(path).await?,
        })
    }

    pub async fn send_request(&mut self, request: &Request) -> Result<()> {
        write_frame(&mut self.stream, request).await
    }

    pub async fn recv_response(&mut self) -> Result<Option<Response>> {
        read_frame(&mut self.stream).await
    }

    pub fn into_stream(self) -> Connection {
        self.stream
    }
}
