//! Live database connections, keyed by a session id the frontend holds on to.

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32},
    },
};

use driver_core::{
    capabilities::Capabilities,
    connect::ConnectConfig,
    connection::{Connection, ServerVersion, TxStatus},
    driver::Driver,
};
use serde::Serialize;
use specta::Type;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::results::ResultStore;

/// `Session::applied_timeout` before the query timeout is first set.
pub const TIMEOUT_UNKNOWN: u32 = u32::MAX;

pub struct Session {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub driver_id: &'static str,
    pub conn: Box<dyn Connection>,
    /// Held for the whole of a console run: one run per session at a time.
    pub run_lock: Mutex<()>,
    /// Set by `cancel_run`; the running script stops before its next statement.
    pub cancelled: AtomicBool,
    /// True while a statement runs or its rows are read. `cancel_run` holds
    /// this lock while it sends the cancel, so a cancel is only sent while
    /// a statement is in flight.
    pub in_flight: Mutex<bool>,
    /// Results the grid pages through.
    pub results: ResultStore,
    /// The query timeout (seconds) last set on the connection.
    pub applied_timeout: AtomicU32,
    /// How this session was opened, to open helper connections (LISTEN) to
    /// the same server. None in tests.
    pub origin: Option<Origin>,
}

pub struct Origin {
    pub driver: Arc<dyn Driver>,
    pub config: ConnectConfig,
}

impl Session {
    pub fn new(connection_id: Uuid, driver_id: &'static str, conn: Box<dyn Connection>) -> Self {
        Self {
            id: Uuid::new_v4(),
            connection_id,
            driver_id,
            conn,
            run_lock: Mutex::new(()),
            cancelled: AtomicBool::new(false),
            in_flight: Mutex::new(false),
            results: ResultStore::default(),
            applied_timeout: AtomicU32::new(TIMEOUT_UNKNOWN),
            origin: None,
        }
    }

    pub fn with_origin(mut self, driver: Arc<dyn Driver>, config: ConnectConfig) -> Self {
        self.origin = Some(Origin { driver, config });
        self
    }

    pub fn info(&self) -> SessionInfo {
        SessionInfo {
            session_id: self.id,
            connection_id: self.connection_id,
            driver_id: self.driver_id,
            server_version: self.conn.server_version().clone(),
            capabilities: self.conn.capabilities().clone(),
            tx_status: self.conn.tx_status(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub connection_id: Uuid,
    pub driver_id: &'static str,
    pub server_version: ServerVersion,
    pub capabilities: Capabilities,
    pub tx_status: TxStatus,
}

#[derive(Default)]
pub struct SessionManager {
    sessions: RwLock<HashMap<Uuid, Arc<Session>>>,
}

impl SessionManager {
    pub async fn insert(&self, session: Session) -> Arc<Session> {
        let session = Arc::new(session);
        self.sessions
            .write()
            .await
            .insert(session.id, Arc::clone(&session));
        session
    }

    pub async fn get(&self, id: Uuid) -> Option<Arc<Session>> {
        self.sessions.read().await.get(&id).cloned()
    }

    pub async fn list(&self) -> Vec<Arc<Session>> {
        self.sessions.read().await.values().cloned().collect()
    }

    /// Removes the session and closes its connection. Returns false if the
    /// id is unknown. Its held results go with it.
    pub async fn close(&self, id: Uuid) -> bool {
        let Some(session) = self.sessions.write().await.remove(&id) else {
            return false;
        };
        match Arc::try_unwrap(session) {
            Ok(session) => session.conn.close().await,
            // Someone (e.g. a running query) still holds it; the connection
            // closes when the last reference is dropped.
            Err(_) => tracing::debug!(%id, "session still in use; closing on drop"),
        }
        true
    }
}
