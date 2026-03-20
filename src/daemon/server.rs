use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, oneshot};
use tokio::time::timeout;
use tracing::debug;

use crate::config;
use crate::error::{AxecError, Result};
use crate::protocol::{Request, Response, read_frame, write_frame};

use super::idle_monitor::ActivityTracker;
use super::session::{Session, SessionEvent, SessionSpec};
use super::session_manager::SessionManager;

pub async fn run(
    listener: UnixListener,
    manager: Arc<SessionManager>,
    activity: ActivityTracker,
    mut shutdown: oneshot::Receiver<()>,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let manager = manager.clone();
                let activity = activity.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle_client(stream, manager, activity).await {
                        debug!(error = %err, "client handling failed");
                    }
                });
            }
        }
    }

    Ok(())
}

async fn handle_client(
    mut stream: UnixStream,
    manager: Arc<SessionManager>,
    activity: ActivityTracker,
) -> Result<()> {
    activity.touch();

    let request = match read_frame::<_, Request>(&mut stream).await? {
        Some(request) => request,
        None => return Ok(()),
    };

    if let Request::Attach { session } = request {
        let session = match manager.get(&session) {
            Ok(session) => session,
            Err(err) => {
                let _ = write_frame(
                    &mut stream,
                    &Response::Error {
                        message: err.to_string(),
                    },
                )
                .await;
                return Ok(());
            }
        };

        write_frame(
            &mut stream,
            &Response::Ack {
                message: "attached".to_string(),
            },
        )
        .await?;

        if let Err(err) = attach_live(stream, session).await {
            debug!(error = %err, "attach handling failed");
        }
        return Ok(());
    }

    let result = route_request(&mut stream, request, manager, activity).await;
    if let Err(err) = result {
        let _ = write_frame(
            &mut stream,
            &Response::Error {
                message: err.to_string(),
            },
        )
        .await;
    }

    Ok(())
}

async fn route_request(
    stream: &mut UnixStream,
    request: Request,
    manager: Arc<SessionManager>,
    activity: ActivityTracker,
) -> Result<()> {
    activity.touch();

    match request {
        Request::Ping => {
            write_frame(stream, &Response::Pong).await?;
        }
        Request::Run {
            command,
            args,
            name,
            timeout,
            terminate,
            cwd,
            env,
        } => {
            let session = manager.create_session(SessionSpec {
                name,
                command,
                args,
                cwd,
                env,
            })?;
            let receiver = session.subscribe();
            write_frame(
                stream,
                &Response::SessionCreated {
                    uuid: session.uuid(),
                    name: session.name(),
                },
            )
            .await?;
            if timeout.is_some() || terminate {
                stream_live(stream, session, receiver, timeout, terminate).await?;
            }
        }
        Request::Cat {
            session,
            follow,
            stderr,
        } => {
            let session = manager.get(&session)?;
            let history = session.history(stderr)?;
            write_frame(stream, &Response::CatOutput { data: history }).await?;
            if follow {
                follow_live(stream, session).await?;
            }
        }
        Request::List => {
            write_frame(
                stream,
                &Response::SessionList {
                    sessions: manager.list_sessions(),
                },
            )
            .await?;
        }
        Request::Input {
            session,
            text,
            timeout,
            terminate,
        } => {
            let session = manager.get(&session)?;
            let receiver = session.subscribe();
            session.write_input(text).await?;
            if timeout.is_some() || terminate {
                stream_live(stream, session, receiver, timeout, terminate).await?;
            } else {
                write_frame(
                    stream,
                    &Response::Ack {
                        message: "ok".to_string(),
                    },
                )
                .await?;
            }
        }
        Request::Signal { session, signal } => {
            let session = manager.get(&session)?;
            session.send_signal(&signal)?;
            write_frame(
                stream,
                &Response::Ack {
                    message: "ok".to_string(),
                },
            )
            .await?;
        }
        Request::Kill { session, all } => {
            let message = if all {
                let count = manager.kill_all();
                format!(
                    "killed {count} session{}",
                    if count == 1 { "" } else { "s" }
                )
            } else {
                let session = manager.get(session.as_deref().ok_or_else(|| {
                    AxecError::Protocol("missing session selector".to_string())
                })?)?;
                let _ = session.kill();
                "ok".to_string()
            };
            write_frame(stream, &Response::Ack { message }).await?;
        }
        Request::Clean => {
            write_frame(
                stream,
                &Response::Cleaned {
                    removed: manager.clean_dead()?,
                },
            )
            .await?;
        }
        Request::Attach { .. } => unreachable!("attach is handled before route_request"),
    }

    Ok(())
}

async fn attach_live(stream: UnixStream, session: Arc<Session>) -> Result<()> {
    let recent_output = session.recent_output();
    let mut receiver = session.subscribe();
    let (mut socket_reader, mut socket_writer) = stream.into_split();

    if !recent_output.is_empty() {
        write_raw(&mut socket_writer, &recent_output).await?;
    }

    let input_session = session.clone();
    let mut input_task = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            let count = socket_reader.read(&mut buf).await?;
            if count == 0 {
                break;
            }
            input_session.write_bytes(buf[..count].to_vec()).await?;
        }

        Ok::<(), AxecError>(())
    });

    loop {
        tokio::select! {
            input_result = &mut input_task => {
                match input_result {
                    Ok(Ok(())) => return Ok(()),
                    Ok(Err(err)) => return Err(err),
                    Err(err) => return Err(err.into()),
                }
            }
            event = receiver.recv() => {
                match event {
                    Ok(SessionEvent::Output { data, .. }) => {
                        if let Err(err) = write_raw(&mut socket_writer, data.as_bytes()).await {
                            if is_client_disconnect(&err) {
                                return Ok(());
                            }
                            return Err(err);
                        }
                    }
                    Ok(SessionEvent::Finished { .. }) => return Ok(()),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                }
            }
        }
    }
}

async fn write_raw<W>(writer: &mut W, bytes: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    writer.write_all(bytes).await?;
    writer.flush().await?;
    Ok(())
}

fn is_client_disconnect(err: &AxecError) -> bool {
    match err {
        AxecError::Io(io_err) => matches!(
            io_err.kind(),
            std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::NotConnected
        ),
        _ => false,
    }
}

async fn stream_live(
    stream: &mut UnixStream,
    session: Arc<Session>,
    mut receiver: broadcast::Receiver<SessionEvent>,
    timeout_secs: Option<u64>,
    terminate: bool,
) -> Result<()> {
    let mut exit_rx = session.exit_receiver();
    let initial_exit = *exit_rx.borrow();
    if let Some(exit_code) = initial_exit {
        write_frame(
            stream,
            &Response::Finished {
                exit_code: Some(exit_code),
                timed_out: false,
                running: false,
            },
        )
        .await?;
        return Ok(());
    }

    let deadline = timeout_secs.map(|seconds| Instant::now() + Duration::from_secs(seconds));

    loop {
        let next_event = if let Some(deadline) = deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                None
            } else {
                (timeout(remaining, receiver.recv()).await).ok()
            }
        } else {
            Some(receiver.recv().await)
        };

        match next_event {
            Some(Ok(SessionEvent::Output {
                data,
                stream: output,
            })) => {
                write_frame(
                    stream,
                    &Response::OutputChunk {
                        data,
                        stream: output,
                    },
                )
                .await?;
            }
            Some(Ok(SessionEvent::Finished { exit_code })) => {
                write_frame(
                    stream,
                    &Response::Finished {
                        exit_code: Some(exit_code),
                        timed_out: false,
                        running: false,
                    },
                )
                .await?;
                return Ok(());
            }
            Some(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Some(Err(broadcast::error::RecvError::Closed)) => {
                let exit_code = *exit_rx.borrow();
                write_frame(
                    stream,
                    &Response::Finished {
                        exit_code,
                        timed_out: false,
                        running: session.is_running(),
                    },
                )
                .await?;
                return Ok(());
            }
            None => {
                let exit_code = *exit_rx.borrow();
                if let Some(exit_code) = exit_code {
                    write_frame(
                        stream,
                        &Response::Finished {
                            exit_code: Some(exit_code),
                            timed_out: false,
                            running: false,
                        },
                    )
                    .await?;
                    return Ok(());
                }

                if terminate {
                    let _ = session.kill();
                    let still_running = exit_rx.borrow().is_none();
                    if still_running {
                        let _ = timeout(
                            Duration::from_secs(config::TERMINATION_GRACE_SECS),
                            exit_rx.changed(),
                        )
                        .await;
                    }
                    write_frame(
                        stream,
                        &Response::Finished {
                            exit_code: Some(124),
                            timed_out: true,
                            running: false,
                        },
                    )
                    .await?;
                } else {
                    write_frame(
                        stream,
                        &Response::Finished {
                            exit_code: None,
                            timed_out: true,
                            running: true,
                        },
                    )
                    .await?;
                }
                return Ok(());
            }
        }
    }
}

async fn follow_live(stream: &mut UnixStream, session: Arc<Session>) -> Result<()> {
    let mut receiver = session.subscribe();
    let exit_rx = session.exit_receiver();
    let initial_exit = *exit_rx.borrow();

    if let Some(exit_code) = initial_exit {
        write_frame(
            stream,
            &Response::Finished {
                exit_code: Some(exit_code),
                timed_out: false,
                running: false,
            },
        )
        .await?;
        return Ok(());
    }

    loop {
        match receiver.recv().await {
            Ok(SessionEvent::Output {
                data,
                stream: output,
            }) => {
                write_frame(
                    stream,
                    &Response::OutputChunk {
                        data,
                        stream: output,
                    },
                )
                .await?;
            }
            Ok(SessionEvent::Finished { exit_code }) => {
                write_frame(
                    stream,
                    &Response::Finished {
                        exit_code: Some(exit_code),
                        timed_out: false,
                        running: false,
                    },
                )
                .await?;
                return Ok(());
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => {
                return Ok(());
            }
        }
    }
}
