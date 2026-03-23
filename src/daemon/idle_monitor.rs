use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::oneshot;
use tokio::time::sleep;

use crate::config;

use super::session_manager::SessionManager;

#[derive(Clone)]
pub struct ActivityTracker {
    last_touch: Arc<AtomicU64>,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            last_touch: Arc::new(AtomicU64::new(now_secs())),
        }
    }

    pub fn touch(&self) {
        self.last_touch.store(now_secs(), Ordering::Relaxed);
    }

    pub fn idle_for(&self) -> Duration {
        Duration::from_secs(now_secs().saturating_sub(self.last_touch.load(Ordering::Relaxed)))
    }
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}

pub fn spawn_idle_monitor(
    manager: Arc<SessionManager>,
    tracker: ActivityTracker,
) -> oneshot::Receiver<()> {
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(config::IDLE_POLL_SECS)).await;
            if manager.running_count() == 0
                && tracker.idle_for() >= Duration::from_secs(config::IDLE_TIMEOUT_SECS)
            {
                let _ = tx.send(());
                break;
            }
        }
    });
    rx
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
