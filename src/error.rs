use thiserror::Error;

pub type Result<T> = std::result::Result<T, AxecError>;

#[derive(Debug, Error)]
pub enum AxecError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Nix(#[from] nix::errno::Errno),
    #[error("home directory is unavailable")]
    HomeDirectoryUnavailable,
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("session name is already in use: {0}")]
    DuplicateSessionName(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}
