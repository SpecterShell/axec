use crate::error::{AxecError, Result};

pub fn spawn_daemon() -> Result<()> {
    Err(AxecError::Unsupported(
        "windows transport is not implemented yet".to_string(),
    ))
}

pub fn send_signal(
    _process_group: Option<i32>,
    _pid: Option<u32>,
    _signal_name: &str,
) -> Result<()> {
    Err(AxecError::Unsupported(
        "windows signal forwarding is not implemented yet".to_string(),
    ))
}

pub fn force_kill(_process_group: Option<i32>, _pid: Option<u32>) -> Result<()> {
    Err(AxecError::Unsupported(
        "windows process killing is not implemented yet".to_string(),
    ))
}
