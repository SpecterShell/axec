use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    TerminateProcess,
};

use crate::error::{AxecError, Result};

pub fn spawn_daemon() -> Result<()> {
    let executable = std::env::current_exe()?;
    let mut command = Command::new(executable);
    command
        .arg("--daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);

    command.spawn()?;
    Ok(())
}

pub fn send_signal(_process_group: Option<i32>, pid: Option<u32>, signal_name: &str) -> Result<()> {
    match parse_signal(signal_name)? {
        WindowsSignal::Interrupt => Err(AxecError::Unsupported(
            "SIGINT is handled by writing Ctrl+C into the pseudo-console".to_string(),
        )),
        WindowsSignal::Terminate | WindowsSignal::Kill => terminate_pid(pid, 1),
    }
}

pub fn force_kill(_process_group: Option<i32>, pid: Option<u32>) -> Result<()> {
    terminate_pid(pid, 1)
}

enum WindowsSignal {
    Interrupt,
    Terminate,
    Kill,
}

fn parse_signal(raw: &str) -> Result<WindowsSignal> {
    if let Ok(number) = raw.parse::<i32>() {
        return match number {
            2 => Ok(WindowsSignal::Interrupt),
            9 => Ok(WindowsSignal::Kill),
            15 => Ok(WindowsSignal::Terminate),
            _ => Err(AxecError::Protocol(format!(
                "unknown signal number: {number}"
            ))),
        };
    }

    let normalized = raw.trim().to_ascii_uppercase();
    let normalized = normalized.strip_prefix("SIG").unwrap_or(&normalized);

    match normalized {
        "INT" | "BREAK" => Ok(WindowsSignal::Interrupt),
        "TERM" => Ok(WindowsSignal::Terminate),
        "KILL" => Ok(WindowsSignal::Kill),
        _ => Err(AxecError::Protocol(format!("unknown signal name: {raw}"))),
    }
}

fn terminate_pid(pid: Option<u32>, exit_code: u32) -> Result<()> {
    let pid = pid.ok_or_else(|| {
        AxecError::Unsupported("session does not expose an OS process id".to_string())
    })?;

    unsafe {
        let handle = OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        );
        if handle.is_null() {
            return Err(std::io::Error::last_os_error().into());
        }

        let terminate_result = TerminateProcess(handle, exit_code);
        let terminate_err = if terminate_result == 0 {
            Some(std::io::Error::last_os_error())
        } else {
            None
        };

        if CloseHandle(handle) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        if let Some(err) = terminate_err {
            return Err(err.into());
        }
    }

    Ok(())
}
