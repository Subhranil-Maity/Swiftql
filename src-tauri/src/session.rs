//! Live database connections, keyed by a session id the frontend holds on to.

use std::{collections::HashMap, sync::Arc};

use driver_core::{
    capabilities::Capabilities,
    connection::{Connection, ServerVersion, TxStatus},
};
use serde::Serialize;
use specta::Type;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct Session {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub driver_id: &'static str,
    pub conn: Box<dyn Connection>,
}

impl Session {
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

    pub async fn list(&self) -> Vec<Arc<Session>> {
        self.sessions.read().await.values().cloned().collect()
    }

    /// Removes the session and closes its connection. Returns false if the
    /// id is unknown.
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
