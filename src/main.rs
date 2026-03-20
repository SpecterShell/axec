#[macro_use]
extern crate rust_i18n;

i18n!("locales", fallback = "en");

#[cfg(unix)]
mod cli;
#[cfg(unix)]
mod client;
#[cfg(unix)]
mod config;
#[cfg(unix)]
mod daemon;
#[cfg(unix)]
mod error;
#[cfg(unix)]
mod i18n;
#[cfg(unix)]
mod paths;
#[cfg(unix)]
mod platform;
#[cfg(unix)]
mod protocol;

#[cfg(unix)]
use std::ffi::OsStr;

#[cfg(unix)]
use tracing_subscriber::EnvFilter;

#[cfg(unix)]
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

#[cfg(not(unix))]
fn main() {
    eprintln!("axec currently supports Unix builds only.");
    std::process::exit(1);
}

#[cfg(unix)]
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .try_init();
}
