pub mod idle_monitor;
pub mod output_buffer;
pub mod server;
pub mod session;
pub mod session_manager;

use std::fs;
use std::sync::Arc;

use crate::error::Result;
use crate::paths;
use crate::transport::Listener;

use self::idle_monitor::{ActivityTracker, spawn_idle_monitor};
use self::session_manager::SessionManager;

pub async fn run() -> Result<()> {
    paths::ensure_base_dirs()?;

    let socket_path = paths::socket_path()?;
    let pid_path = paths::pid_path()?;

    #[cfg(unix)]
    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }

    fs::write(&pid_path, format!("{}\n", std::process::id()))?;

    let listener = Listener::bind(&socket_path)?;
    let manager = Arc::new(SessionManager::new());
    let activity = ActivityTracker::new();
    let shutdown = spawn_idle_monitor(manager.clone(), activity.clone());

    let result = server::run(listener, manager, activity, shutdown).await;

    #[cfg(unix)]
    let _ = fs::remove_file(&socket_path);
    let _ = fs::remove_file(&pid_path);

    result
}
