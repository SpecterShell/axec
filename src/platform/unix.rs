use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use nix::sys::signal::{self, Signal};
use nix::unistd::{Pid, setsid};

use crate::error::{AxecError, Result};

pub fn spawn_daemon() -> Result<()> {
    let executable = std::env::current_exe()?;
    let mut command = Command::new(executable);
    command
        .arg("--daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        command.pre_exec(|| {
            setsid().map_err(|err| std::io::Error::from_raw_os_error(err as i32))?;
            Ok(())
        });
    }

    command.spawn()?;
    Ok(())
}

pub fn send_signal(process_group: Option<i32>, pid: Option<u32>, signal_name: &str) -> Result<()> {
    let signal = parse_signal(signal_name)?;
    if let Some(process_group) = process_group {
        signal::killpg(Pid::from_raw(process_group), signal)?;
        return Ok(());
    }
    if let Some(pid) = pid {
        signal::kill(Pid::from_raw(pid as i32), signal)?;
        return Ok(());
    }
    Err(AxecError::Unsupported(
        "session does not expose an OS process id".to_string(),
    ))
}

pub fn force_kill(process_group: Option<i32>, pid: Option<u32>) -> Result<()> {
    if let Some(process_group) = process_group {
        signal::killpg(Pid::from_raw(process_group), Signal::SIGKILL)?;
        return Ok(());
    }
    if let Some(pid) = pid {
        signal::kill(Pid::from_raw(pid as i32), Signal::SIGKILL)?;
        return Ok(());
    }
    Err(AxecError::Unsupported(
        "session does not expose an OS process id".to_string(),
    ))
}

fn parse_signal(raw: &str) -> Result<Signal> {
    if let Ok(number) = raw.parse::<i32>() {
        return Signal::try_from(number)
            .map_err(|_| AxecError::Protocol(format!("unknown signal number: {number}")));
    }

    let normalized = raw.trim().to_ascii_uppercase();
    let normalized = normalized.strip_prefix("SIG").unwrap_or(&normalized);

    let signal = match normalized {
        "HUP" => Signal::SIGHUP,
        "INT" => Signal::SIGINT,
        "QUIT" => Signal::SIGQUIT,
        "TERM" => Signal::SIGTERM,
        "KILL" => Signal::SIGKILL,
        "USR1" => Signal::SIGUSR1,
        "USR2" => Signal::SIGUSR2,
        "STOP" => Signal::SIGSTOP,
        "CONT" => Signal::SIGCONT,
        _ => {
            return Err(AxecError::Protocol(format!("unknown signal name: {raw}")));
        }
    };

    Ok(signal)
}
