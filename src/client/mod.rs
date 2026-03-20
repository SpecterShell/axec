pub mod commands;
pub mod connection;

use crate::cli::Cli;
use crate::error::Result;

pub async fn run(cli: Cli) -> Result<i32> {
    commands::run(cli).await
}
