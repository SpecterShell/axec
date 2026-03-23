#[macro_use]
extern crate rust_i18n;

i18n!("locales", fallback = "en");

pub mod cli;
pub mod client;
pub mod config;
pub mod daemon;
pub mod error;
pub mod i18n;
pub mod paths;
pub mod platform;
pub mod protocol;
pub mod repl;
pub mod terminal;
pub mod transport;
