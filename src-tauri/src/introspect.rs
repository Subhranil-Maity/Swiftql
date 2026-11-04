//! Reading a session's database structure: for autocomplete, and later the
//! schema browser.

use driver_core::{
    error::{DbError, ErrorKind},
    introspec::{IntrospectScope, SchemaSnapshot},
};
use tauri::State;
use uuid::Uuid;

use crate::session::SessionManager;

/// Runs on the session's connection, so it waits behind a running query and
/// fails inside a failed transaction like any other statement.
#[tauri::command]
#[specta::specta]
pub async fn introspect(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    scope: IntrospectScope,
) -> Result<SchemaSnapshot, DbError> {
    let session = sessions.get(session_id).await.ok_or_else(|| {
        DbError::new(
            ErrorKind::NotFound,
            format!("session {session_id} not found"),
        )
    })?;
    session.conn.introspect(scope).await
}
