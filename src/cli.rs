use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command as ClapCommand, value_parser};

use crate::i18n;
use crate::protocol::{EnvVar, SessionBackend};

#[derive(Debug, Clone)]
pub struct Cli {
    pub json: bool,
    pub command: Command,
}

#[derive(Debug, Clone)]
pub enum Command {
    Run(RunArgs),
    Cat(CatArgs),
    Output(OutputArgs),
    List,
    Input(InputArgs),
    Signal(SignalArgs),
    Kill(KillArgs),
    Attach(SessionArgs),
    Clean,
}

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub name: Option<String>,
    pub timeout: Option<u64>,
    pub stopword: Option<String>,
    pub terminate: bool,
    pub backend: SessionBackend,
    pub cwd: Option<PathBuf>,
    pub env: Vec<EnvVar>,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CatArgs {
    pub session: Option<String>,
    pub follow: bool,
    pub stderr: bool,
}

#[derive(Debug, Clone)]
pub struct OutputArgs {
    pub session: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InputArgs {
    pub session: Option<String>,
    pub timeout: Option<u64>,
    pub stopword: Option<String>,
    pub terminate: bool,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SignalArgs {
    pub session: Option<String>,
    pub signal: String,
}

#[derive(Debug, Clone)]
pub struct SessionArgs {
    pub session: String,
}

#[derive(Debug, Clone)]
pub struct KillArgs {
    pub session: Option<String>,
    pub all: bool,
}

pub fn parse() -> Result<Cli, clap::Error> {
    let matches = build_command().try_get_matches()?;
    from_matches(matches)
}

fn build_command() -> ClapCommand {
    ClapCommand::new("axec")
        .about(i18n::text("help.app_about").to_string())
        .arg_required_else_help(true)
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .help(i18n::text("help.json").to_string())
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommand(build_run_command())
        .subcommand(build_cat_command())
        .subcommand(build_output_command())
        .subcommand(
            ClapCommand::new("list")
                .about(i18n::text("help.list_about").to_string())
                .alias("sessions"),
        )
        .subcommand(build_input_command())
        .subcommand(build_signal_command())
        .subcommand(build_kill_command())
        .subcommand(build_attach_command())
        .subcommand(
            ClapCommand::new("clean")
                .alias("clear")
                .about(i18n::text("help.clean_about").to_string()),
        )
}

fn build_run_command() -> ClapCommand {
    ClapCommand::new("run")
        .about(i18n::text("help.run_about").to_string())
        .alias("exec")
        .trailing_var_arg(true)
        .arg(
            Arg::new("name")
                .long("name")
                .value_name("NAME")
                .help(i18n::text("help.name").to_string()),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .value_parser(value_parser!(u64))
                .help(i18n::text("help.timeout").to_string()),
        )
        .arg(
            Arg::new("stopword")
                .long("stopword")
                .value_name("REGEX")
                .help(i18n::text("help.stopword").to_string()),
        )
        .arg(
            Arg::new("terminate")
                .long("terminate")
                .action(ArgAction::SetTrue)
                .help(i18n::text("help.terminate").to_string()),
        )
        .arg(
            Arg::new("backend")
                .long("backend")
                .value_name("KIND")
                .default_value("pty")
                .value_parser(["pty", "pipe", "auto"])
                .help(i18n::text("help.backend").to_string()),
        )
        .arg(
            Arg::new("cwd")
                .long("cwd")
                .value_name("DIR")
                .value_parser(value_parser!(PathBuf))
                .help(i18n::text("help.cwd").to_string()),
        )
        .arg(
            Arg::new("env")
                .long("env")
                .value_name("K=V")
                .action(ArgAction::Append)
                .help(i18n::text("help.env").to_string()),
        )
        .arg(
            Arg::new("command")
                .required(true)
                .value_name("CMD")
                .help(i18n::text("help.command").to_string()),
        )
        .arg(
            Arg::new("args")
                .num_args(0..)
                .allow_hyphen_values(true)
                .value_name("ARGS"),
        )
}

fn build_cat_command() -> ClapCommand {
    ClapCommand::new("cat")
        .about(i18n::text("help.cat_about").to_string())
        .arg(optional_session_arg())
        .arg(
            Arg::new("follow")
                .long("follow")
                .action(ArgAction::SetTrue)
                .help(i18n::text("help.follow").to_string()),
        )
        .arg(
            Arg::new("stderr")
                .long("stderr")
                .action(ArgAction::SetTrue)
                .help(i18n::text("help.stderr").to_string()),
        )
}

fn build_input_command() -> ClapCommand {
    ClapCommand::new("input")
        .about(i18n::text("help.input_about").to_string())
        .arg(optional_session_arg())
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .value_parser(value_parser!(u64))
                .help(i18n::text("help.timeout").to_string()),
        )
        .arg(
            Arg::new("stopword")
                .long("stopword")
                .value_name("REGEX")
                .help(i18n::text("help.stopword").to_string()),
        )
        .arg(
            Arg::new("terminate")
                .long("terminate")
                .action(ArgAction::SetTrue)
                .help(i18n::text("help.terminate").to_string()),
        )
        .arg(
            Arg::new("text")
                .required(true)
                .allow_hyphen_values(true)
                .value_name("TEXT")
                .help(i18n::text("help.input_text").to_string()),
        )
}

fn build_output_command() -> ClapCommand {
    ClapCommand::new("output")
        .about(i18n::text("help.output_about").to_string())
        .arg(optional_session_arg())
}

fn build_signal_command() -> ClapCommand {
    ClapCommand::new("signal")
        .about(i18n::text("help.signal_about").to_string())
        .arg(optional_session_arg())
        .arg(
            Arg::new("signal")
                .required(true)
                .value_name("SIGNAL")
                .help(i18n::text("help.signal").to_string()),
        )
}

fn build_kill_command() -> ClapCommand {
    ClapCommand::new("kill")
        .about(i18n::text("help.kill_about").to_string())
        .alias("terminate")
        .arg(optional_session_arg().conflicts_with("all"))
        .arg(
            Arg::new("all")
                .long("all")
                .action(ArgAction::SetTrue)
                .help(i18n::text("help.all").to_string())
                .conflicts_with("session"),
        )
        .group(
            ArgGroup::new("kill_target")
                .args(["session", "all"])
                .required(true),
        )
}

fn build_attach_command() -> ClapCommand {
    ClapCommand::new("attach")
        .about(i18n::text("help.attach_about").to_string())
        .arg(required_session_arg())
}

fn required_session_arg() -> Arg {
    Arg::new("session")
        .long("session")
        .required(true)
        .value_name("UUID|NAME")
        .help(i18n::text("help.session").to_string())
}

fn optional_session_arg() -> Arg {
    Arg::new("session")
        .long("session")
        .required(false)
        .value_name("UUID|NAME")
        .help(i18n::text("help.session").to_string())
}

fn from_matches(matches: ArgMatches) -> Result<Cli, clap::Error> {
    let json = matches.get_flag("json");
    let Some((name, submatches)) = matches.subcommand() else {
        return Err(clap::Error::raw(
            ErrorKind::MissingSubcommand,
            i18n::text("help.missing_subcommand").to_string(),
        ));
    };

    let command = match name {
        "run" | "exec" => Command::Run(RunArgs {
            name: submatches.get_one::<String>("name").cloned(),
            timeout: submatches.get_one::<u64>("timeout").copied(),
            stopword: submatches.get_one::<String>("stopword").cloned(),
            terminate: submatches.get_flag("terminate"),
            backend: parse_backend(submatches),
            cwd: submatches.get_one::<PathBuf>("cwd").cloned(),
            env: parse_env_vars(submatches.get_many::<String>("env"))?,
            command: required_value(submatches, "command")?,
            args: values(submatches, "args"),
        }),
        "cat" => Command::Cat(CatArgs {
            session: submatches.get_one::<String>("session").cloned(),
            follow: submatches.get_flag("follow"),
            stderr: submatches.get_flag("stderr"),
        }),
        "output" => Command::Output(OutputArgs {
            session: submatches.get_one::<String>("session").cloned(),
        }),
        "list" | "sessions" => Command::List,
        "input" => Command::Input(InputArgs {
            session: submatches.get_one::<String>("session").cloned(),
            timeout: submatches.get_one::<u64>("timeout").copied(),
            stopword: submatches.get_one::<String>("stopword").cloned(),
            terminate: submatches.get_flag("terminate"),
            text: required_value(submatches, "text")?,
        }),
        "signal" => Command::Signal(SignalArgs {
            session: submatches.get_one::<String>("session").cloned(),
            signal: required_value(submatches, "signal")?,
        }),
        "kill" | "terminate" => Command::Kill(KillArgs {
            session: submatches.get_one::<String>("session").cloned(),
            all: submatches.get_flag("all"),
        }),
        "attach" => Command::Attach(SessionArgs {
            session: required_value(submatches, "session")?,
        }),
        "clean" | "clear" => Command::Clean,
        _ => {
            return Err(clap::Error::raw(
                ErrorKind::UnknownArgument,
                i18n::text("help.unknown_command").to_string(),
            ));
        }
    };

    Ok(Cli { json, command })
}

fn parse_env_vars<'a>(
    values: Option<clap::parser::ValuesRef<'a, String>>,
) -> Result<Vec<EnvVar>, clap::Error> {
    values
        .into_iter()
        .flatten()
        .map(|raw| match raw.split_once('=') {
            Some((key, value)) if !key.is_empty() => Ok(EnvVar {
                key: key.to_string(),
                value: value.to_string(),
            }),
            _ => Err(clap::Error::raw(
                ErrorKind::ValueValidation,
                format!("{}: {raw}", i18n::text("help.invalid_env")),
            )),
        })
        .collect()
}

fn required_value(matches: &ArgMatches, name: &str) -> Result<String, clap::Error> {
    matches
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| clap::Error::raw(ErrorKind::MissingRequiredArgument, name.to_string()))
}

fn values(matches: &ArgMatches, name: &str) -> Vec<String> {
    matches
        .get_many::<String>(name)
        .into_iter()
        .flatten()
        .cloned()
        .collect()
}

fn parse_backend(matches: &ArgMatches) -> SessionBackend {
    match matches
        .get_one::<String>("backend")
        .map(String::as_str)
        .unwrap_or("pty")
    {
        "pty" => SessionBackend::Pty,
        "pipe" => SessionBackend::Pipe,
        "auto" => SessionBackend::Auto,
        _ => unreachable!("backend value is validated by clap"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, build_command, from_matches};
    use crate::protocol::SessionBackend;

    #[test]
    fn run_defaults_backend_to_pty() {
        let matches = build_command()
            .try_get_matches_from(["axec", "run", "echo"])
            .expect("run command should parse");
        let cli = from_matches(matches).expect("cli should parse");

        match cli.command {
            Command::Run(args) => assert_eq!(args.backend, SessionBackend::Pty),
            other => panic!("expected run command, got {other:?}"),
        }
    }

    #[test]
    fn run_accepts_explicit_auto_backend() {
        let matches = build_command()
            .try_get_matches_from(["axec", "run", "--backend", "auto", "echo"])
            .expect("run command should parse");
        let cli = from_matches(matches).expect("cli should parse");

        match cli.command {
            Command::Run(args) => assert_eq!(args.backend, SessionBackend::Auto),
            other => panic!("expected run command, got {other:?}"),
        }
    }
}
