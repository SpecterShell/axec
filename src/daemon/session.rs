use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use portable_pty::{Child, ChildKiller, CommandBuilder, PtySize, native_pty_system};
#[cfg(unix)]
use std::process::{Child as StdChild, Command as StdCommand, Stdio};
use tokio::sync::{broadcast, watch};
#[cfg(windows)]
use tracing::debug;
use uuid::Uuid;

#[cfg(windows)]
use conpty::Process;
#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::process::{Child as StdChild, Command, Stdio};
#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
};

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::platform;
use crate::protocol::{
    EnvVar, OutputStream, SessionBackend, SessionInfo, SessionMeta, SessionStatus,
};

use super::output_buffer::OutputBuffer;

#[derive(Debug, Clone)]
pub struct SessionSpec {
    pub name: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub backend: SessionBackend,
    pub cwd: Option<PathBuf>,
    pub env: Vec<EnvVar>,
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Output {
        data: String,
        stream: OutputStream,
        stdout_end: Option<u64>,
    },
    Finished {
        exit_code: i32,
    },
}

trait SessionWaiter: Send {
    fn wait(&mut self) -> Result<i32>;
}

trait SessionKiller: Send + Sync {
    fn kill(&mut self) -> Result<()>;
}

struct SpawnedSession {
    pid: Option<u32>,
    process_group: Option<i32>,
    backend: SessionBackend,
    stdout_reader: Box<dyn Read + Send>,
    stderr_reader: Option<Box<dyn Read + Send>>,
    writer: Box<dyn Write + Send>,
    waiter: Box<dyn SessionWaiter>,
    killer: Option<Box<dyn SessionKiller>>,
    #[cfg(windows)]
    interrupt_via_stdin: bool,
    carriage_return_newlines: bool,
}

pub struct Session {
    meta: Arc<Mutex<SessionMeta>>,
    meta_path: PathBuf,
    session_dir: PathBuf,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    killer: Option<Arc<Mutex<Box<dyn SessionKiller>>>>,
    #[cfg(windows)]
    interrupt_via_stdin: bool,
    carriage_return_newlines: bool,
    stdout: Arc<OutputBuffer>,
    stderr: Arc<OutputBuffer>,
    stdout_cursor: Arc<Mutex<u64>>,
    events: broadcast::Sender<SessionEvent>,
    exit_tx: watch::Sender<Option<i32>>,
}

impl Session {
    pub fn spawn(spec: SessionSpec) -> Result<Arc<Self>> {
        let id = Uuid::new_v4();
        let session_dir = paths::session_dir(&id)?;
        fs::create_dir_all(&session_dir)?;

        let stdout = Arc::new(OutputBuffer::new(
            paths::session_stdout_log(&id)?,
            config::OUTPUT_RING_BYTES,
        )?);
        let stderr = Arc::new(OutputBuffer::new(
            paths::session_stderr_log(&id)?,
            config::OUTPUT_RING_BYTES,
        )?);

        let spawned = spawn_process(&spec)?;
        let started_at = now_timestamp_millis();

        let meta = SessionMeta {
            uuid: id,
            name: spec.name.clone(),
            command: spec.command.clone(),
            args: spec.args.clone(),
            cwd: spec.cwd.clone(),
            env: spec.env.clone(),
            pid: spawned.pid,
            process_group: spawned.process_group,
            backend: spawned.backend,
            started_at,
            exited_at: None,
            status: SessionStatus::Running,
        };

        let meta_path = paths::session_meta_path(&id)?;
        persist_meta(&meta_path, &meta)?;

        let (events, _) = broadcast::channel(1024);
        let (exit_tx, _) = watch::channel(None);

        let session = Arc::new(Self {
            meta: Arc::new(Mutex::new(meta)),
            meta_path,
            session_dir,
            writer: Arc::new(Mutex::new(spawned.writer)),
            killer: spawned.killer.map(|killer| Arc::new(Mutex::new(killer))),
            #[cfg(windows)]
            interrupt_via_stdin: spawned.interrupt_via_stdin,
            carriage_return_newlines: spawned.carriage_return_newlines,
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            stdout_cursor: Arc::new(Mutex::new(0)),
            events,
            exit_tx,
        });

        spawn_reader_thread(
            spawned.stdout_reader,
            stdout,
            session.events.clone(),
            OutputStream::Stdout,
        );
        if let Some(stderr_reader) = spawned.stderr_reader {
            spawn_reader_thread(
                stderr_reader,
                stderr,
                session.events.clone(),
                OutputStream::Stderr,
            );
        }
        spawn_wait_thread(
            spawned.waiter,
            session.meta.clone(),
            session.meta_path.clone(),
            session.events.clone(),
            session.exit_tx.clone(),
        );

        Ok(session)
    }

    pub fn uuid(&self) -> Uuid {
        self.meta.lock().expect("session meta mutex poisoned").uuid
    }

    pub fn name(&self) -> Option<String> {
        self.meta
            .lock()
            .expect("session meta mutex poisoned")
            .name
            .clone()
    }

    pub fn directory(&self) -> &Path {
        &self.session_dir
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self.meta
                .lock()
                .expect("session meta mutex poisoned")
                .status,
            SessionStatus::Running
        )
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.events.subscribe()
    }

    pub fn exit_receiver(&self) -> watch::Receiver<Option<i32>> {
        self.exit_tx.subscribe()
    }

    pub fn info(&self) -> SessionInfo {
        let meta = self
            .meta
            .lock()
            .expect("session meta mutex poisoned")
            .clone();
        SessionInfo {
            uuid: meta.uuid,
            name: meta.name,
            command: display_command(&meta.command, &meta.args),
            cwd: meta.cwd,
            pid: meta.pid,
            backend: meta.backend,
            started_at: meta.started_at,
            exited_at: meta.exited_at,
            status: meta.status,
        }
    }

    pub fn history(&self, stderr: bool) -> Result<String> {
        if stderr {
            self.stderr.read_all_string()
        } else {
            self.stdout.read_all_string()
        }
    }

    pub fn recent_output(&self) -> Vec<u8> {
        self.stdout.read_recent_bytes()
    }

    pub fn unread_stdout(&self) -> Result<(String, u64)> {
        let start = *self
            .stdout_cursor
            .lock()
            .expect("session stdout cursor mutex poisoned");
        self.stdout.read_string_from(start)
    }

    pub fn mark_stdout_consumed(&self, end: u64) {
        let mut cursor = self
            .stdout_cursor
            .lock()
            .expect("session stdout cursor mutex poisoned");
        *cursor = (*cursor).max(end);
    }

    pub async fn write_input(&self, mut text: String) -> Result<()> {
        normalize_input_text(&mut text, self.carriage_return_newlines);

        let writer = self.writer.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut writer = writer.lock().expect("session writer mutex poisoned");
            writer.write_all(text.as_bytes())?;
            writer.flush()?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    pub async fn write_bytes(&self, bytes: Vec<u8>) -> Result<()> {
        let writer = self.writer.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut writer = writer.lock().expect("session writer mutex poisoned");
            writer.write_all(&bytes)?;
            writer.flush()?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    pub fn send_signal(&self, signal: &str) -> Result<()> {
        #[cfg(windows)]
        if self.interrupt_via_stdin && is_interrupt_signal(signal) {
            let mut writer = self.writer.lock().expect("session writer mutex poisoned");
            writer.write_all(&[0x03])?;
            writer.flush()?;
            return Ok(());
        }

        let meta = self
            .meta
            .lock()
            .expect("session meta mutex poisoned")
            .clone();
        platform::send_signal(meta.process_group, meta.pid, signal)
    }

    pub fn kill(&self) -> Result<()> {
        #[cfg(windows)]
        if let Some(killer) = &self.killer {
            let mut killer = killer.lock().expect("session killer mutex poisoned");
            if killer.kill().is_ok() {
                return Ok(());
            }
        }

        let meta = self
            .meta
            .lock()
            .expect("session meta mutex poisoned")
            .clone();

        if let Err(err) = platform::force_kill(meta.process_group, meta.pid) {
            if let Some(killer) = &self.killer {
                let mut killer = killer.lock().expect("session killer mutex poisoned");
                killer.kill()?;
                return Ok(());
            }
            return Err(err);
        }

        Ok(())
    }
}

fn display_command(command: &str, args: &[String]) -> String {
    if args.is_empty() {
        command.to_string()
    } else {
        format!("{command} {}", args.join(" "))
    }
}

fn persist_meta(path: &Path, meta: &SessionMeta) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(meta)?)?;
    Ok(())
}

fn spawn_reader_thread(
    mut reader: Box<dyn Read + Send>,
    buffer: Arc<OutputBuffer>,
    events: broadcast::Sender<SessionEvent>,
    stream: OutputStream,
) {
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(count) => {
                    let chunk = &buf[..count];
                    let buffer_end = buffer.append(chunk).ok();
                    let _ = events.send(SessionEvent::Output {
                        data: String::from_utf8_lossy(chunk).to_string(),
                        stream,
                        stdout_end: if matches!(stream, OutputStream::Stdout) {
                            buffer_end
                        } else {
                            None
                        },
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });
}

fn spawn_wait_thread(
    mut waiter: Box<dyn SessionWaiter>,
    meta: Arc<Mutex<SessionMeta>>,
    meta_path: PathBuf,
    events: broadcast::Sender<SessionEvent>,
    exit_tx: watch::Sender<Option<i32>>,
) {
    thread::spawn(move || {
        let exit_code = waiter.wait().unwrap_or(1);

        {
            let mut meta = meta.lock().expect("session meta mutex poisoned");
            meta.exited_at = Some(now_timestamp_millis());
            meta.status = SessionStatus::Exited { exit_code };
            let _ = persist_meta(&meta_path, &meta);
        }

        let _ = exit_tx.send(Some(exit_code));
        let _ = events.send(SessionEvent::Finished { exit_code });
    });
}

fn now_timestamp_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(unix)]
fn spawn_process(spec: &SessionSpec) -> Result<SpawnedSession> {
    match spec.backend {
        SessionBackend::Pty | SessionBackend::Auto => spawn_unix_pty_process(spec),
        SessionBackend::Pipe => spawn_unix_piped_process(spec),
    }
}

#[cfg(unix)]
fn spawn_unix_pty_process(spec: &SessionSpec) -> Result<SpawnedSession> {
    let pty = native_pty_system().openpty(PtySize::default())?;
    let mut builder = CommandBuilder::new(&spec.command);
    for arg in &spec.args {
        builder.arg(arg);
    }
    if let Some(cwd) = &spec.cwd {
        builder.cwd(cwd);
    }
    for env in &spec.env {
        builder.env(&env.key, &env.value);
    }

    let child = pty.slave.spawn_command(builder)?;
    let pid = child.process_id();
    let process_group = pty.master.process_group_leader();
    let killer = child.clone_killer();
    let reader = pty.master.try_clone_reader()?;
    let writer = pty.master.take_writer()?;

    Ok(SpawnedSession {
        pid,
        process_group,
        backend: SessionBackend::Pty,
        stdout_reader: reader,
        stderr_reader: None,
        writer,
        waiter: Box::new(UnixSessionWaiter { child }),
        killer: Some(Box::new(UnixSessionKiller { killer })),
        carriage_return_newlines: false,
    })
}

#[cfg(unix)]
fn spawn_unix_piped_process(spec: &SessionSpec) -> Result<SpawnedSession> {
    let mut command = StdCommand::new(&spec.command);
    command
        .args(&spec.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = &spec.cwd {
        command.current_dir(cwd);
    }
    for env in &spec.env {
        command.env(&env.key, &env.value);
    }

    let mut child = command.spawn()?;
    let pid = Some(child.id());
    let writer = Box::new(
        child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("child stdin was not piped"))?,
    );
    let stdout_reader = Box::new(
        child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("child stdout was not piped"))?,
    );
    let stderr_reader = Some(Box::new(
        child
            .stderr
            .take()
            .ok_or_else(|| std::io::Error::other("child stderr was not piped"))?,
    ) as Box<dyn Read + Send>);

    Ok(SpawnedSession {
        pid,
        process_group: None,
        backend: SessionBackend::Pipe,
        stdout_reader,
        stderr_reader,
        writer,
        waiter: Box::new(UnixPipeSessionWaiter { child }),
        killer: None,
        carriage_return_newlines: false,
    })
}

#[cfg(unix)]
struct UnixSessionWaiter {
    child: Box<dyn Child + Send + Sync>,
}

#[cfg(unix)]
impl SessionWaiter for UnixSessionWaiter {
    fn wait(&mut self) -> Result<i32> {
        Ok(self.child.wait()?.exit_code() as i32)
    }
}

#[cfg(unix)]
struct UnixPipeSessionWaiter {
    child: StdChild,
}

#[cfg(unix)]
impl SessionWaiter for UnixPipeSessionWaiter {
    fn wait(&mut self) -> Result<i32> {
        Ok(self.child.wait()?.code().unwrap_or(1))
    }
}

#[cfg(unix)]
struct UnixSessionKiller {
    killer: Box<dyn ChildKiller + Send + Sync>,
}

#[cfg(unix)]
impl SessionKiller for UnixSessionKiller {
    fn kill(&mut self) -> Result<()> {
        self.killer.kill()?;
        Ok(())
    }
}

#[cfg(windows)]
fn spawn_process(spec: &SessionSpec) -> Result<SpawnedSession> {
    match spec.backend {
        SessionBackend::Pty => spawn_windows_pty_process(spec),
        SessionBackend::Pipe => spawn_windows_piped_process(spec),
        SessionBackend::Auto => {
            if should_use_windows_pipes(spec) {
                spawn_windows_piped_process(spec)
            } else {
                spawn_windows_pty_process(spec)
            }
        }
    }
}

#[cfg(windows)]
fn spawn_windows_pty_process(spec: &SessionSpec) -> Result<SpawnedSession> {
    let mut command = Command::new(&spec.command);
    command.args(&spec.args);
    if let Some(cwd) = &spec.cwd {
        command.current_dir(cwd);
    }
    for env in &spec.env {
        command.env(&env.key, &env.value);
    }

    let mut process = Process::spawn(command).map_err(std::io::Error::from)?;
    let pid = Some(process.pid());
    let reader = Box::new(process.output().map_err(std::io::Error::from)?);
    let writer = Box::new(process.input().map_err(std::io::Error::from)?);
    let killer = pid.and_then(|pid| match WindowsJobKiller::new(pid) {
        Ok(killer) => Some(Box::new(killer) as Box<dyn SessionKiller>),
        Err(err) => {
            debug!(error = %err, pid, "unable to assign session to a job object");
            None
        }
    });

    Ok(SpawnedSession {
        pid,
        process_group: None,
        backend: SessionBackend::Pty,
        stdout_reader: reader,
        stderr_reader: None,
        writer,
        waiter: Box::new(WindowsSessionWaiter { process }),
        killer,
        interrupt_via_stdin: true,
        carriage_return_newlines: true,
    })
}

#[cfg(windows)]
fn spawn_windows_piped_process(spec: &SessionSpec) -> Result<SpawnedSession> {
    let mut command = Command::new(&spec.command);
    command
        .args(&spec.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW);
    if let Some(cwd) = &spec.cwd {
        command.current_dir(cwd);
    }
    for env in &spec.env {
        command.env(&env.key, &env.value);
    }

    let mut child = command.spawn()?;
    let pid = Some(child.id());
    let writer = Box::new(
        child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("child stdin was not piped"))?,
    );
    let stdout_reader = Box::new(
        child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("child stdout was not piped"))?,
    );
    let stderr_reader = Some(Box::new(
        child
            .stderr
            .take()
            .ok_or_else(|| std::io::Error::other("child stderr was not piped"))?,
    ) as Box<dyn Read + Send>);

    Ok(SpawnedSession {
        pid,
        process_group: None,
        backend: SessionBackend::Pipe,
        stdout_reader,
        stderr_reader,
        writer,
        waiter: Box::new(WindowsPipeSessionWaiter { child }),
        killer: None,
        interrupt_via_stdin: false,
        carriage_return_newlines: false,
    })
}

#[cfg(windows)]
struct WindowsSessionWaiter {
    process: Process,
}

#[cfg(windows)]
impl SessionWaiter for WindowsSessionWaiter {
    fn wait(&mut self) -> Result<i32> {
        Ok(self.process.wait(None).map_err(std::io::Error::from)? as i32)
    }
}

#[cfg(windows)]
struct WindowsPipeSessionWaiter {
    child: StdChild,
}

#[cfg(windows)]
impl SessionWaiter for WindowsPipeSessionWaiter {
    fn wait(&mut self) -> Result<i32> {
        Ok(self.child.wait()?.code().unwrap_or(1))
    }
}

#[cfg(windows)]
struct WindowsJobKiller {
    job: HANDLE,
}

#[cfg(windows)]
impl WindowsJobKiller {
    fn new(pid: u32) -> Result<Self> {
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }

            let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
            if process.is_null() {
                let err = std::io::Error::last_os_error();
                let _ = CloseHandle(job);
                return Err(err.into());
            }

            let assign_result = AssignProcessToJobObject(job, process);
            let assign_err = if assign_result == 0 {
                Some(std::io::Error::last_os_error())
            } else {
                None
            };

            if CloseHandle(process) == 0 {
                let err = std::io::Error::last_os_error();
                let _ = CloseHandle(job);
                return Err(err.into());
            }

            if let Some(err) = assign_err {
                let _ = CloseHandle(job);
                return Err(err.into());
            }

            Ok(Self { job })
        }
    }
}

#[cfg(windows)]
impl SessionKiller for WindowsJobKiller {
    fn kill(&mut self) -> Result<()> {
        unsafe {
            if TerminateJobObject(self.job, 1) == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }

        Ok(())
    }
}

#[cfg(windows)]
impl Drop for WindowsJobKiller {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.job);
        }
    }
}

#[cfg(windows)]
unsafe impl Send for WindowsJobKiller {}

#[cfg(windows)]
unsafe impl Sync for WindowsJobKiller {}

#[cfg(windows)]
fn is_interrupt_signal(raw: &str) -> bool {
    if let Ok(number) = raw.parse::<i32>() {
        return number == 2;
    }

    let normalized = raw.trim().to_ascii_uppercase();
    let normalized = normalized.strip_prefix("SIG").unwrap_or(&normalized);
    matches!(normalized, "INT" | "BREAK")
}

#[cfg(unix)]
fn normalize_input_text(text: &mut String, _carriage_return_newlines: bool) {
    if !text.ends_with('\n') && !text.ends_with('\r') {
        text.push('\n');
    }
}

#[cfg(windows)]
fn normalize_input_text(text: &mut String, carriage_return_newlines: bool) {
    if carriage_return_newlines {
        let normalized = text.replace("\r\n", "\r").replace('\n', "\r");
        *text = normalized;
        if !text.ends_with('\r') {
            text.push('\r');
        }
    } else if !text.ends_with('\n') && !text.ends_with('\r') {
        text.push('\n');
    }
}

#[cfg(windows)]
fn should_use_windows_pipes(spec: &SessionSpec) -> bool {
    let command = basename(&spec.command).to_ascii_lowercase();
    let args = spec
        .args
        .iter()
        .map(|arg| arg.to_ascii_lowercase())
        .collect::<Vec<_>>();

    match command.as_str() {
        "cmd" | "cmd.exe" => args.iter().any(|arg| arg == "/c"),
        "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => args.iter().any(|arg| {
            matches!(
                arg.as_str(),
                "-c" | "-command" | "-file" | "-encodedcommand" | "-ec"
            )
        }),
        "bash" | "bash.exe" | "sh" | "zsh" | "fish" => args.iter().any(|arg| arg == "-c"),
        "python" | "python.exe" | "python3" | "python3.exe" | "ipython" | "ipython.exe" => {
            !args.is_empty() && !args.iter().any(|arg| arg == "-i")
        }
        "node" | "node.exe" => !args.is_empty(),
        "psql" | "psql.exe" => false,
        _ => true,
    }
}

#[cfg(windows)]
fn basename(command: &str) -> &str {
    std::path::Path::new(OsStr::new(command))
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(command)
}
