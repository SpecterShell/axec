#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{force_kill, send_signal, spawn_daemon};

#[cfg(not(unix))]
mod windows;
#[cfg(not(unix))]
pub use windows::{force_kill, send_signal, spawn_daemon};
