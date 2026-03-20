use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{Child, ChildKiller, CommandBuilder, PtySize, native_pty_system};
use tokio::sync::{broadcast, watch};
use uuid::Uuid;

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::platform;
use crate::protocol::{EnvVar, OutputStream, SessionInfo, SessionMeta, SessionStatus};

use super::output_buffer::OutputBuffer;

#[derive(Debug, Clone)]
pub struct SessionSpec {
    pub name: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<EnvVar>,
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Output { data: String, stream: OutputStream },
    Finished { exit_code: i32 },
}

pub struct Session {
    meta: Arc<Mutex<SessionMeta>>,
    meta_path: PathBuf,
    session_dir: PathBuf,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    killer: Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>,
    stdout: Arc<OutputBuffer>,
    stderr: Arc<OutputBuffer>,
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

        let meta = SessionMeta {
            uuid: id,
            name: spec.name.clone(),
            command: spec.command.clone(),
            args: spec.args.clone(),
            cwd: spec.cwd.clone(),
            env: spec.env.clone(),
            pid,
            process_group,
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
            writer: Arc::new(Mutex::new(writer)),
            killer: Arc::new(Mutex::new(killer)),
            stdout: stdout.clone(),
            stderr,
            events,
            exit_tx,
        });

        spawn_reader_thread(reader, stdout, session.events.clone());
        spawn_wait_thread(
            child,
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

    pub async fn write_input(&self, mut text: String) -> Result<()> {
        if !text.ends_with('\n') && !text.ends_with('\r') {
            text.push('\n');
        }

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
        let meta = self
            .meta
            .lock()
            .expect("session meta mutex poisoned")
            .clone();
        platform::send_signal(meta.process_group, meta.pid, signal)
    }

    pub fn kill(&self) -> Result<()> {
        let meta = self
            .meta
            .lock()
            .expect("session meta mutex poisoned")
            .clone();
        if let Err(err) = platform::force_kill(meta.process_group, meta.pid) {
            let mut killer = self.killer.lock().expect("session killer mutex poisoned");
            killer.kill()?;
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
    stdout: Arc<OutputBuffer>,
    events: broadcast::Sender<SessionEvent>,
) {
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(count) => {
                    let chunk = &buf[..count];
                    let _ = stdout.append(chunk);
                    let _ = events.send(SessionEvent::Output {
                        data: String::from_utf8_lossy(chunk).to_string(),
                        stream: OutputStream::Stdout,
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });
}

fn spawn_wait_thread(
    mut child: Box<dyn Child + Send + Sync>,
    meta: Arc<Mutex<SessionMeta>>,
    meta_path: PathBuf,
    events: broadcast::Sender<SessionEvent>,
    exit_tx: watch::Sender<Option<i32>>,
) {
    thread::spawn(move || {
        let exit_code = match child.wait() {
            Ok(status) => status.exit_code() as i32,
            Err(_) => 1,
        };

        {
            let mut meta = meta.lock().expect("session meta mutex poisoned");
            meta.status = SessionStatus::Exited { exit_code };
            let _ = persist_meta(&meta_path, &meta);
        }

        let _ = exit_tx.send(Some(exit_code));
        let _ = events.send(SessionEvent::Finished { exit_code });
    });
}
