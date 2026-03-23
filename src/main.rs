use std::ffi::OsStr;

use axec::{cli, client, daemon, i18n};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    init_tracing();
    i18n::init_locale();

    let result = if std::env::args_os().nth(1).as_deref() == Some(OsStr::new("--daemon")) {
        daemon::run().await.map(|()| 0)
    } else {
        match cli::parse() {
            Ok(cli) => client::run(cli).await,
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
