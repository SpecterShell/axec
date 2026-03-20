use std::io::{self, IsTerminal, Write};

use nix::sys::termios::{
    SetArg, SpecialCharacterIndices, Termios, cfmakeraw, tcgetattr, tcsetattr,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::cli::{CatArgs, Cli, Command, InputArgs, KillArgs, RunArgs, SessionArgs, SignalArgs};
use crate::client::connection::DaemonConnection;
use crate::error::{AxecError, Result};
use crate::protocol::{Request, Response, SessionInfo};

#[derive(Debug, Clone, Copy)]
struct FinishedState {
    exit_code: Option<i32>,
    timed_out: bool,
    running: bool,
}

pub async fn run(cli: Cli) -> Result<i32> {
    let json = cli.json;
    match cli.command {
        Command::Run(args) => run_command(args, json).await,
        Command::Cat(args) => cat_command(args, json).await,
        Command::List => list_command(json).await,
        Command::Input(args) => input_command(args, json).await,
        Command::Signal(args) => signal_command(args, json).await,
        Command::Kill(args) => kill_command(args, json).await,
        Command::Attach(args) => attach_command(args, json).await,
        Command::Clean => clean_command(json).await,
    }
}

async fn run_command(args: RunArgs, json: bool) -> Result<i32> {
    let mut connection = DaemonConnection::connect().await?;
    connection
        .send_request(&Request::Run {
            command: args.command.clone(),
            args: args.args.clone(),
            name: args.name.clone(),
            timeout: args.timeout,
            terminate: args.terminate,
            cwd: args.cwd.clone(),
            env: args.env.clone(),
        })
        .await?;

    let response = expect_response(&mut connection).await?;
    match &response {
        Response::SessionCreated { .. } => emit_response(&response, json)?,
        _ => {
            return Err(AxecError::Protocol(
                "daemon returned an unexpected response to run".to_string(),
            ));
        }
    }

    if args.timeout.is_some() || args.terminate {
        let finished = drain_stream(&mut connection, json).await?;
        Ok(match finished {
            Some(state) if state.timed_out && args.terminate => 124,
            Some(state) if args.terminate && !state.running => state.exit_code.unwrap_or(1),
            _ => 0,
        })
    } else {
        Ok(0)
    }
}

async fn cat_command(args: CatArgs, json: bool) -> Result<i32> {
    let mut connection = DaemonConnection::connect().await?;
    connection
        .send_request(&Request::Cat {
            session: args.session,
            follow: args.follow,
            stderr: args.stderr,
        })
        .await?;

    while let Some(response) = connection.recv_response().await? {
        match response {
            Response::Error { message } => return Err(AxecError::Protocol(message)),
            Response::Finished { .. } => {
                if json {
                    emit_response(&response, true)?;
                }
                break;
            }
            other => emit_response(&other, json)?,
        }
    }

    Ok(0)
}

async fn list_command(json: bool) -> Result<i32> {
    let mut connection = DaemonConnection::connect().await?;
    connection.send_request(&Request::List).await?;

    match expect_response(&mut connection).await? {
        Response::SessionList { sessions } => {
            if json {
                emit_json(&Response::SessionList {
                    sessions: sessions.clone(),
                })?;
            } else {
                print_sessions(&sessions)?;
            }
            Ok(0)
        }
        other => Err(AxecError::Protocol(format!(
            "unexpected list response: {other:?}"
        ))),
    }
}

async fn input_command(args: InputArgs, json: bool) -> Result<i32> {
    let mut connection = DaemonConnection::connect().await?;
    let text = if args.text == "-" {
        read_stdin_text().await?
    } else {
        args.text.clone()
    };

    connection
        .send_request(&Request::Input {
            session: args.session,
            text,
            timeout: args.timeout,
            terminate: args.terminate,
        })
        .await?;

    if args.timeout.is_some() || args.terminate {
        let finished = drain_stream(&mut connection, json).await?;
        Ok(match finished {
            Some(state) if state.timed_out && args.terminate => 124,
            Some(state) if args.terminate && !state.running => state.exit_code.unwrap_or(1),
            _ => 0,
        })
    } else {
        let response = expect_response(&mut connection).await?;
        emit_response(&response, json)?;
        Ok(0)
    }
}

async fn signal_command(args: SignalArgs, json: bool) -> Result<i32> {
    let mut connection = DaemonConnection::connect().await?;
    connection
        .send_request(&Request::Signal {
            session: args.session,
            signal: args.signal,
        })
        .await?;

    let response = expect_response(&mut connection).await?;
    emit_response(&response, json)?;
    Ok(0)
}

async fn kill_command(args: KillArgs, json: bool) -> Result<i32> {
    let mut connection = DaemonConnection::connect().await?;
    connection
        .send_request(&Request::Kill {
            session: args.session,
            all: args.all,
        })
        .await?;

    let response = expect_response(&mut connection).await?;
    emit_response(&response, json)?;
    Ok(0)
}

async fn attach_command(args: SessionArgs, json: bool) -> Result<i32> {
    if json {
        return Err(AxecError::Unsupported(
            "--json is not supported with attach".to_string(),
        ));
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(AxecError::Unsupported(
            "attach requires an interactive terminal".to_string(),
        ));
    }

    let mut connection = DaemonConnection::connect().await?;
    connection
        .send_request(&Request::Attach {
            session: args.session,
        })
        .await?;

    let response = expect_response(&mut connection).await?;
    match response {
        Response::Ack { .. } => {}
        other => {
            return Err(AxecError::Protocol(format!(
                "unexpected attach response: {other:?}"
            )));
        }
    }

    let _terminal = RawTerminalGuard::new()?;
    let stream = connection.into_stream();
    let (mut socket_reader, mut socket_writer) = stream.into_split();
    let mut input_task = tokio::spawn(async move {
        let mut stdin = tokio::io::stdin();
        let mut buf = [0u8; 1024];

        loop {
            let count = stdin.read(&mut buf).await?;
            if count == 0 {
                break;
            }

            if let Some(position) = buf[..count].iter().position(|byte| *byte == 0x1c) {
                if position > 0 {
                    socket_writer.write_all(&buf[..position]).await?;
                    socket_writer.flush().await?;
                }
                break;
            }

            socket_writer.write_all(&buf[..count]).await?;
            socket_writer.flush().await?;
        }

        Ok::<(), AxecError>(())
    });

    let mut stdout = tokio::io::stdout();
    let mut buf = [0u8; 4096];

    loop {
        tokio::select! {
            input_result = &mut input_task => {
                match input_result {
                    Ok(Ok(())) => break,
                    Ok(Err(err)) => return Err(err),
                    Err(err) => return Err(err.into()),
                }
            }
            read_result = socket_reader.read(&mut buf) => {
                let count = read_result?;
                if count == 0 {
                    break;
                }
                stdout.write_all(&buf[..count]).await?;
                stdout.flush().await?;
            }
        }
    }

    Ok(0)
}

async fn clean_command(json: bool) -> Result<i32> {
    let mut connection = DaemonConnection::connect().await?;
    connection.send_request(&Request::Clean).await?;

    let response = expect_response(&mut connection).await?;
    emit_response(&response, json)?;
    Ok(0)
}

async fn drain_stream(
    connection: &mut DaemonConnection,
    json: bool,
) -> Result<Option<FinishedState>> {
    let mut finished = None;

    while let Some(response) = connection.recv_response().await? {
        match response {
            Response::Error { message } => return Err(AxecError::Protocol(message)),
            Response::Finished {
                exit_code,
                timed_out,
                running,
            } => {
                finished = Some(FinishedState {
                    exit_code,
                    timed_out,
                    running,
                });
                if json {
                    emit_json(&Response::Finished {
                        exit_code,
                        timed_out,
                        running,
                    })?;
                }
                break;
            }
            other => emit_response(&other, json)?,
        }
    }

    Ok(finished)
}

async fn expect_response(connection: &mut DaemonConnection) -> Result<Response> {
    match connection.recv_response().await? {
        Some(Response::Error { message }) => Err(AxecError::Protocol(message)),
        Some(response) => Ok(response),
        None => Err(AxecError::Protocol(
            "daemon closed the connection unexpectedly".to_string(),
        )),
    }
}

async fn read_stdin_text() -> Result<String> {
    let mut input = Vec::new();
    tokio::io::stdin().read_to_end(&mut input).await?;
    Ok(String::from_utf8_lossy(&input).to_string())
}

fn emit_response(response: &Response, json: bool) -> Result<()> {
    if json {
        emit_json(response)
    } else {
        emit_plain(response)
    }
}

fn emit_json(response: &Response) -> Result<()> {
    println!("{}", serde_json::to_string(response)?);
    Ok(())
}

fn emit_plain(response: &Response) -> Result<()> {
    match response {
        Response::SessionCreated { uuid, .. } => {
            println!("{uuid}");
        }
        Response::OutputChunk { data, stream } => match stream {
            crate::protocol::OutputStream::Stdout => {
                let mut stdout = io::stdout().lock();
                stdout.write_all(data.as_bytes())?;
                stdout.flush()?;
            }
            crate::protocol::OutputStream::Stderr => {
                let mut stderr = io::stderr().lock();
                stderr.write_all(data.as_bytes())?;
                stderr.flush()?;
            }
        },
        Response::CatOutput { data } => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(data.as_bytes())?;
            stdout.flush()?;
        }
        Response::SessionList { sessions } => {
            print_sessions(sessions)?;
        }
        Response::Ack { message } => {
            if !message.is_empty() {
                println!("{message}");
            }
        }
        Response::Cleaned { removed } => {
            println!("{removed}");
        }
        Response::Pong | Response::Finished { .. } => {}
        Response::Error { message } => return Err(AxecError::Protocol(message.clone())),
    }
    Ok(())
}

fn print_sessions(sessions: &[SessionInfo]) -> Result<()> {
    let mut stdout = io::stdout().lock();
    let uuid_width = 36usize;
    let name_width = sessions
        .iter()
        .map(|session| session.name.as_deref().unwrap_or("-").len())
        .max()
        .unwrap_or(0)
        .max("NAME".len());
    let status_width = sessions
        .iter()
        .map(|session| session.status.to_string().len())
        .max()
        .unwrap_or(0)
        .max("STATUS".len());

    writeln!(
        stdout,
        "{:<uuid_width$}  {:<name_width$}  {:<status_width$}  COMMAND",
        "UUID",
        "NAME",
        "STATUS",
        uuid_width = uuid_width,
        name_width = name_width,
        status_width = status_width,
    )?;
    for session in sessions {
        let status = session.status.to_string();
        writeln!(
            stdout,
            "{:<uuid_width$}  {:<name_width$}  {:<status_width$}  {}",
            session.uuid,
            session.name.as_deref().unwrap_or("-"),
            status,
            session.command,
            uuid_width = uuid_width,
            name_width = name_width,
            status_width = status_width,
        )?;
    }
    stdout.flush()?;
    Ok(())
}

struct RawTerminalGuard {
    original: Termios,
}

impl RawTerminalGuard {
    fn new() -> Result<Self> {
        let stdin = io::stdin();
        let mut raw = tcgetattr(&stdin)?;
        let original = raw.clone();

        cfmakeraw(&mut raw);
        raw.control_chars[SpecialCharacterIndices::VMIN as usize] = 1;
        raw.control_chars[SpecialCharacterIndices::VTIME as usize] = 0;
        tcsetattr(&stdin, SetArg::TCSANOW, &raw)?;

        Ok(Self { original })
    }
}

impl Drop for RawTerminalGuard {
    fn drop(&mut self) {
        let stdin = io::stdin();
        let _ = tcsetattr(&stdin, SetArg::TCSANOW, &self.original);
    }
}
