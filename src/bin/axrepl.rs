use std::ffi::OsStr;
use std::io::Write;
use std::path::PathBuf;

use axec::client::connection::DaemonConnection;
use axec::daemon;
use axec::error::{AxecError, Result};
use axec::i18n;
use axec::protocol::{EnvVar, Request, Response, SessionInfo, SessionStatus};
use axec::repl::{
    ReplDriver, infer_driver, infer_session_driver, strip_completion_output, wrap_script,
    write_session_driver,
};
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command as ClapCommand, value_parser};
use tokio::io::AsyncReadExt;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Debug)]
enum Command {
    Run(RunArgs),
    Input(InputArgs),
    List,
    Kill(KillArgs),
    Clean,
}

#[derive(Debug)]
struct Cli {
    json: bool,
    command: Command,
}

#[derive(Debug)]
struct RunArgs {
    name: Option<String>,
    driver: Option<ReplDriver>,
    cwd: Option<PathBuf>,
    env: Vec<EnvVar>,
    command: String,
    args: Vec<String>,
}

#[derive(Debug)]
struct InputArgs {
    session: Option<String>,
    driver: Option<ReplDriver>,
    text: String,
}

#[derive(Debug)]
struct KillArgs {
    session: Option<String>,
    all: bool,
}

#[derive(Debug)]
struct ReplSession {
    info: SessionInfo,
    driver: ReplDriver,
}

#[tokio::main]
async fn main() {
    init_tracing();
    i18n::init_locale();

    let result = if std::env::args_os().nth(1).as_deref() == Some(OsStr::new("--daemon")) {
        daemon::run().await.map(|()| 0)
    } else {
        match parse() {
            Ok(cli) => run(cli).await,
            Err(err) => err.exit(),
        }
    };

    match result {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .try_init();
}

fn parse() -> std::result::Result<Cli, clap::Error> {
    let matches = build_command().try_get_matches()?;
    let json = matches.get_flag("json");
    let Some((name, submatches)) = matches.subcommand() else {
        unreachable!("subcommand_required is set")
    };

    let command = match name {
        "run" => Command::Run(RunArgs {
            name: submatches.get_one::<String>("name").cloned(),
            driver: parse_driver(submatches),
            cwd: submatches.get_one::<PathBuf>("cwd").cloned(),
            env: parse_env_vars(submatches.get_many::<String>("env"))?,
            command: required_value(submatches, "command")?,
            args: values(submatches, "args"),
        }),
        "input" => Command::Input(InputArgs {
            session: submatches.get_one::<String>("session").cloned(),
            driver: parse_driver(submatches),
            text: required_value(submatches, "text")?,
        }),
        "list" => Command::List,
        "kill" => Command::Kill(KillArgs {
            session: submatches.get_one::<String>("session").cloned(),
            all: submatches.get_flag("all"),
        }),
        "clean" => Command::Clean,
        _ => unreachable!("all subcommands are enumerated"),
    };

    Ok(Cli { json, command })
}

fn build_command() -> ClapCommand {
    ClapCommand::new("axrepl")
        .about("REPL-focused fork of axec with completion-aware input")
        .arg_required_else_help(true)
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .help("Emit structured JSON responses")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommand(build_run_command())
        .subcommand(build_input_command())
        .subcommand(ClapCommand::new("list").about("List known REPL sessions"))
        .subcommand(build_kill_command())
        .subcommand(
            ClapCommand::new("clean").about("Remove exited sessions and their on-disk state"),
        )
}

fn build_run_command() -> ClapCommand {
    ClapCommand::new("run")
        .about("Start a REPL session")
        .arg(
            Arg::new("name")
                .long("name")
                .value_name("NAME")
                .help("Optional unique name for the session"),
        )
        .arg(
            Arg::new("driver")
                .long("driver")
                .value_name("DRIVER")
                .value_parser(["python", "node", "bash", "zsh"])
                .help("REPL driver override when the command cannot be inferred"),
        )
        .arg(
            Arg::new("cwd")
                .long("cwd")
                .value_name("DIR")
                .value_parser(value_parser!(PathBuf))
                .help("Working directory for the spawned REPL"),
        )
        .arg(
            Arg::new("env")
                .long("env")
                .value_name("K=V")
                .action(ArgAction::Append)
                .help("Environment override in K=V form"),
        )
        .arg(
            Arg::new("command")
                .required(true)
                .value_name("CMD")
                .help("REPL command to execute"),
        )
        .arg(
            Arg::new("args")
                .num_args(0..)
                .allow_hyphen_values(true)
                .value_name("ARGS"),
        )
}

fn build_input_command() -> ClapCommand {
    ClapCommand::new("input")
        .about("Send a script to a REPL session and wait until the REPL finishes processing it")
        .arg(
            Arg::new("session")
                .long("session")
                .value_name("UUID|NAME")
                .help("Session UUID, unique UUID prefix, or active name"),
        )
        .arg(
            Arg::new("driver")
                .long("driver")
                .value_name("DRIVER")
                .value_parser(["python", "node", "bash", "zsh"])
                .help("Override the detected REPL driver for this request"),
        )
        .arg(
            Arg::new("text")
                .required(true)
                .allow_hyphen_values(true)
                .value_name("TEXT|-")
                .help("Script text to send, or - to read stdin"),
        )
}

fn build_kill_command() -> ClapCommand {
    ClapCommand::new("kill")
        .about("Force-kill a running REPL session")
        .arg(
            Arg::new("session")
                .long("session")
                .value_name("UUID|NAME")
                .help("Session UUID, unique UUID prefix, or active name")
                .conflicts_with("all"),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(ArgAction::SetTrue)
                .help("Target all tracked running sessions")
                .conflicts_with("session"),
        )
        .group(
            ArgGroup::new("kill_target")
                .args(["session", "all"])
                .required(true),
        )
}

async fn run(cli: Cli) -> Result<i32> {
    match cli.command {
        Command::Run(args) => run_repl(args, cli.json).await,
        Command::Input(args) => input_repl(args, cli.json).await,
        Command::List => list_repls(cli.json).await,
        Command::Kill(args) => kill_repl(args, cli.json).await,
        Command::Clean => clean_repls(cli.json).await,
    }
}

async fn run_repl(args: RunArgs, json: bool) -> Result<i32> {
    let driver = args
        .driver
        .or_else(|| infer_driver(&args.command))
        .ok_or_else(|| {
            AxecError::Unsupported(
                "unable to infer a supported REPL driver; pass --driver explicitly".to_string(),
            )
        })?;

    let mut connection = DaemonConnection::connect().await?;
    connection
        .send_request(&Request::Run {
            command: args.command,
            args: args.args,
            name: args.name,
            timeout: None,
            stopword: None,
            terminate: false,
            backend: axec::protocol::SessionBackend::Pty,
            cwd: args.cwd,
            env: args.env,
        })
        .await?;

    match expect_response(&mut connection).await? {
        Response::SessionCreated { uuid, name } => {
            write_session_driver(&uuid, driver)?;
            if json {
                emit_json(&serde_json::json!({
                    "uuid": uuid,
                    "name": name,
                    "driver": driver,
                }))?;
            } else {
                println!("{uuid}");
            }
            Ok(0)
        }
        other => Err(AxecError::Protocol(format!(
            "unexpected run response: {other:?}"
        ))),
    }
}

async fn input_repl(args: InputArgs, json: bool) -> Result<i32> {
    let session = resolve_repl_session(args.session.as_deref(), args.driver).await?;
    let text = if args.text == "-" {
        read_stdin_text().await?
    } else {
        args.text
    };

    let marker = format!("__AXREPL_DONE_{}__", Uuid::new_v4().simple());
    let wrapped = wrap_script(session.driver, &text, &marker)?;

    let mut connection = DaemonConnection::connect().await?;
    connection
        .send_request(&Request::Input {
            session: Some(session.info.uuid.to_string()),
            text: wrapped.clone(),
            timeout: None,
            stopword: Some(regex::escape(&marker)),
            terminate: false,
        })
        .await?;

    let mut output = String::new();
    loop {
        match expect_response(&mut connection).await? {
            Response::OutputChunk { data, .. } => output.push_str(&data),
            Response::Finished { running, .. } => {
                if !output.contains(&marker) {
                    return Err(AxecError::Protocol(if running {
                        "input finished before the REPL completion marker was observed".to_string()
                    } else {
                        "the REPL session exited before returning its completion marker".to_string()
                    }));
                }

                let cleaned = strip_completion_output(&wrapped, &marker, &output);
                if json {
                    emit_json(&serde_json::json!({
                        "session": session.info.uuid,
                        "driver": session.driver,
                        "data": cleaned,
                    }))?;
                } else {
                    print!("{cleaned}");
                    std::io::stdout().flush()?;
                }
                return Ok(0);
            }
            Response::Error { message } => return Err(AxecError::Protocol(message)),
            other => {
                return Err(AxecError::Protocol(format!(
                    "unexpected input response: {other:?}"
                )));
            }
        }
    }
}

async fn list_repls(json: bool) -> Result<i32> {
    let sessions = fetch_sessions().await?;
    let mut repl_sessions = Vec::new();
    for info in sessions {
        if let Some(driver) = infer_session_driver(&info.uuid)? {
            repl_sessions.push(ReplSession { info, driver });
        }
    }

    repl_sessions.sort_by_key(|session| session.info.started_at);
    repl_sessions.reverse();

    if json {
        emit_json(&repl_sessions_to_json(&repl_sessions))?;
    } else {
        print_repl_sessions(&repl_sessions)?;
    }

    Ok(0)
}

async fn kill_repl(args: KillArgs, json: bool) -> Result<i32> {
    let mut connection = DaemonConnection::connect().await?;
    connection
        .send_request(&Request::Kill {
            session: args.session,
            all: args.all,
        })
        .await?;

    match expect_response(&mut connection).await? {
        Response::Ack { message } => {
            if json {
                emit_json(&serde_json::json!({ "message": message }))?;
            } else {
                println!("{message}");
            }
            Ok(0)
        }
        other => Err(AxecError::Protocol(format!(
            "unexpected kill response: {other:?}"
        ))),
    }
}

async fn clean_repls(json: bool) -> Result<i32> {
    let mut connection = DaemonConnection::connect().await?;
    connection.send_request(&Request::Clean).await?;

    match expect_response(&mut connection).await? {
        Response::Cleaned { removed } => {
            if json {
                emit_json(&serde_json::json!({ "removed": removed }))?;
            } else {
                println!("removed {removed} session(s)");
            }
            Ok(0)
        }
        other => Err(AxecError::Protocol(format!(
            "unexpected clean response: {other:?}"
        ))),
    }
}

async fn fetch_sessions() -> Result<Vec<SessionInfo>> {
    let mut connection = DaemonConnection::connect().await?;
    connection.send_request(&Request::List).await?;
    match expect_response(&mut connection).await? {
        Response::SessionList { sessions } => Ok(sessions),
        other => Err(AxecError::Protocol(format!(
            "unexpected list response: {other:?}"
        ))),
    }
}

async fn resolve_repl_session(
    selector: Option<&str>,
    driver_override: Option<ReplDriver>,
) -> Result<ReplSession> {
    let sessions = fetch_sessions().await?;

    let info = match selector {
        Some(selector) => select_session(&sessions, selector)?,
        None => {
            let mut latest: Option<SessionInfo> = None;
            for session in &sessions {
                if driver_override.is_none() && infer_session_driver(&session.uuid)?.is_none() {
                    continue;
                }
                if latest
                    .as_ref()
                    .is_none_or(|current| session.started_at > current.started_at)
                {
                    latest = Some(session.clone());
                }
            }
            latest.ok_or_else(|| AxecError::SessionNotFound("latest".to_string()))?
        }
    };

    let driver = driver_override
        .or(infer_session_driver(&info.uuid)?)
        .ok_or_else(|| {
            AxecError::Unsupported(
                "unable to determine the REPL driver for this session; pass --driver explicitly"
                    .to_string(),
            )
        })?;

    Ok(ReplSession { info, driver })
}

fn select_session(sessions: &[SessionInfo], selector: &str) -> Result<SessionInfo> {
    if let Some(session) = sessions
        .iter()
        .find(|session| session.uuid.to_string() == selector)
    {
        return Ok(session.clone());
    }

    if let Some(session) = sessions
        .iter()
        .find(|session| session.name.as_deref() == Some(selector))
    {
        return Ok(session.clone());
    }

    let mut prefix_matches = sessions
        .iter()
        .filter(|session| session.uuid.to_string().starts_with(selector));
    let first = prefix_matches.next().cloned();
    match (first, prefix_matches.next()) {
        (Some(session), None) => Ok(session),
        _ => Err(AxecError::SessionNotFound(selector.to_string())),
    }
}

fn repl_sessions_to_json(sessions: &[ReplSession]) -> serde_json::Value {
    serde_json::Value::Array(
        sessions
            .iter()
            .map(|session| {
                serde_json::json!({
                    "uuid": session.info.uuid,
                    "name": session.info.name,
                    "driver": session.driver,
                    "status": session_status(&session.info.status),
                    "command": session.info.command,
                })
            })
            .collect(),
    )
}

fn print_repl_sessions(sessions: &[ReplSession]) -> Result<()> {
    for session in sessions {
        let name = session.info.name.as_deref().unwrap_or("-");
        println!(
            "{}\t{}\t{}\t{}\t{}",
            session.info.uuid,
            name,
            serde_json::to_string(&session.driver)?.trim_matches('"'),
            session_status(&session.info.status),
            session.info.command,
        );
    }
    Ok(())
}

fn session_status(status: &SessionStatus) -> String {
    match status {
        SessionStatus::Running => "running".to_string(),
        SessionStatus::Exited { exit_code } => format!("exited({exit_code})"),
    }
}

async fn read_stdin_text() -> Result<String> {
    let mut stdin = tokio::io::stdin();
    let mut buf = Vec::new();
    stdin.read_to_end(&mut buf).await?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

async fn expect_response(connection: &mut DaemonConnection) -> Result<Response> {
    connection
        .recv_response()
        .await?
        .ok_or_else(|| AxecError::Protocol("daemon closed the connection unexpectedly".to_string()))
}

fn emit_json(value: &serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn parse_env_vars<'a>(
    values: Option<clap::parser::ValuesRef<'a, String>>,
) -> std::result::Result<Vec<EnvVar>, clap::Error> {
    values
        .into_iter()
        .flatten()
        .map(|raw| match raw.split_once('=') {
            Some((key, value)) if !key.is_empty() => Ok(EnvVar {
                key: key.to_string(),
                value: value.to_string(),
            }),
            _ => Err(clap::Error::raw(
                clap::error::ErrorKind::ValueValidation,
                format!("--env expects K=V: {raw}"),
            )),
        })
        .collect()
}

fn required_value(matches: &ArgMatches, name: &str) -> std::result::Result<String, clap::Error> {
    matches
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| clap::Error::raw(clap::error::ErrorKind::MissingRequiredArgument, name))
}

fn values(matches: &ArgMatches, name: &str) -> Vec<String> {
    matches
        .get_many::<String>(name)
        .into_iter()
        .flatten()
        .cloned()
        .collect()
}

fn parse_driver(matches: &ArgMatches) -> Option<ReplDriver> {
    match matches.get_one::<String>("driver").map(String::as_str) {
        Some("python") => Some(ReplDriver::Python),
        Some("node") => Some(ReplDriver::Node),
        Some("bash") => Some(ReplDriver::Bash),
        Some("zsh") => Some(ReplDriver::Zsh),
        Some(_) => unreachable!("driver value is validated by clap"),
        None => None,
    }
}
