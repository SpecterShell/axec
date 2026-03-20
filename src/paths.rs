use std::fs;
use std::path::PathBuf;

use uuid::Uuid;

use crate::config;
use crate::error::{AxecError, Result};

pub fn root_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|path| path.join(format!(".{}", config::APP_NAME)))
        .ok_or(AxecError::HomeDirectoryUnavailable)
}

pub fn runtime_dir() -> Result<PathBuf> {
    let base = dirs::runtime_dir().unwrap_or(root_dir()?);
    Ok(base.join(config::APP_NAME))
}

pub fn sessions_dir() -> Result<PathBuf> {
    Ok(root_dir()?.join(config::SESSION_DIR_NAME))
}

pub fn ensure_base_dirs() -> Result<()> {
    fs::create_dir_all(root_dir()?)?;
    fs::create_dir_all(runtime_dir()?)?;
    fs::create_dir_all(sessions_dir()?)?;
    Ok(())
}

pub fn socket_path() -> Result<PathBuf> {
    Ok(runtime_dir()?.join(config::SOCKET_FILE_NAME))
}

pub fn pid_path() -> Result<PathBuf> {
    Ok(runtime_dir()?.join(config::PID_FILE_NAME))
}

pub fn session_dir(id: &Uuid) -> Result<PathBuf> {
    Ok(sessions_dir()?.join(id.to_string()))
}

pub fn session_stdout_log(id: &Uuid) -> Result<PathBuf> {
    Ok(session_dir(id)?.join(config::STDOUT_LOG_NAME))
}

pub fn session_stderr_log(id: &Uuid) -> Result<PathBuf> {
    Ok(session_dir(id)?.join(config::STDERR_LOG_NAME))
}

pub fn session_meta_path(id: &Uuid) -> Result<PathBuf> {
    Ok(session_dir(id)?.join(config::META_FILE_NAME))
}
