use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::error::{AxecError, Result};
use crate::protocol::SessionInfo;

use super::session::{Session, SessionSpec};

struct SessionManagerInner {
    sessions: HashMap<Uuid, Arc<Session>>,
    names: HashMap<String, Uuid>,
}

pub struct SessionManager {
    inner: Mutex<SessionManagerInner>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(SessionManagerInner {
                sessions: HashMap::new(),
                names: HashMap::new(),
            }),
        }
    }

    pub fn create_session(&self, spec: SessionSpec) -> Result<Arc<Session>> {
        let requested_name = spec.name.clone();
        {
            let mut inner = self.inner.lock().expect("session manager mutex poisoned");
            Self::prune_names_locked(&mut inner);
            if let Some(name) = requested_name.as_deref()
                && inner.names.contains_key(name)
            {
                return Err(AxecError::DuplicateSessionName(name.to_string()));
            }
        }

        let session = Session::spawn(spec)?;
        let uuid = session.uuid();
        let name = session.name();

        let mut inner = self.inner.lock().expect("session manager mutex poisoned");
        inner.sessions.insert(uuid, session.clone());
        if let Some(name) = name {
            inner.names.insert(name, uuid);
        }
        Ok(session)
    }

    pub fn get(&self, selector: &str) -> Result<Arc<Session>> {
        let mut inner = self.inner.lock().expect("session manager mutex poisoned");
        Self::prune_names_locked(&mut inner);

        if let Ok(uuid) = Uuid::parse_str(selector) {
            return inner
                .sessions
                .get(&uuid)
                .cloned()
                .ok_or_else(|| AxecError::SessionNotFound(selector.to_string()));
        }

        if let Some(uuid) = inner.names.get(selector).copied() {
            return inner
                .sessions
                .get(&uuid)
                .cloned()
                .ok_or_else(|| AxecError::SessionNotFound(selector.to_string()));
        }

        let mut matches = inner
            .sessions
            .iter()
            .filter(|(uuid, _)| uuid.to_string().starts_with(selector))
            .map(|(_, session)| session.clone());

        match (matches.next(), matches.next()) {
            (Some(session), None) => Ok(session),
            (Some(_), Some(_)) => Err(AxecError::Protocol(format!(
                "session selector is ambiguous: {selector}"
            ))),
            _ => Err(AxecError::SessionNotFound(selector.to_string())),
        }
    }

    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        let mut inner = self.inner.lock().expect("session manager mutex poisoned");
        Self::prune_names_locked(&mut inner);
        let mut sessions = inner
            .sessions
            .values()
            .map(|session| session.info())
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.uuid.cmp(&right.uuid));
        sessions
    }

    pub fn running_count(&self) -> usize {
        self.inner
            .lock()
            .expect("session manager mutex poisoned")
            .sessions
            .values()
            .filter(|session| session.is_running())
            .count()
    }

    pub fn clean_dead(&self) -> Result<usize> {
        let dead = {
            let mut inner = self.inner.lock().expect("session manager mutex poisoned");
            Self::prune_names_locked(&mut inner);
            let dead = inner
                .sessions
                .iter()
                .filter(|(_, session)| !session.is_running())
                .map(|(uuid, session)| (*uuid, session.directory().to_path_buf()))
                .collect::<Vec<_>>();
            for (uuid, _) in &dead {
                inner.sessions.remove(uuid);
            }
            dead
        };

        for (_, path) in &dead {
            let _ = fs::remove_dir_all(path);
        }

        Ok(dead.len())
    }

    pub fn kill_all(&self) -> usize {
        let sessions = self
            .inner
            .lock()
            .expect("session manager mutex poisoned")
            .sessions
            .values()
            .filter(|session| session.is_running())
            .cloned()
            .collect::<Vec<_>>();

        sessions
            .into_iter()
            .filter(|session| session.kill().is_ok())
            .count()
    }

    fn prune_names_locked(inner: &mut SessionManagerInner) {
        inner.names.retain(|_, uuid| {
            inner
                .sessions
                .get(uuid)
                .is_some_and(|session| session.is_running())
        });
    }
}
