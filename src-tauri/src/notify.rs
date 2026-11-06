//! LISTEN / NOTIFY. The Notifications view listens on a dedicated listener
//! connection per session (opened like the session, tunnel included), so it
//! never waits behind a console query. `LISTEN` typed in the console is
//! delivered on the session's own connection; both reach the UI as
//! `NotificationReceived` events.

use std::collections::{BTreeSet, HashMap};

use driver_core::{
    connection::Connection,
    error::{DbError, ErrorKind},
    notify::Notification,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, State};
use tauri_specta::Event;
use tokio::sync::{Mutex, broadcast::error::RecvError};
use uuid::Uuid;

use crate::session::SessionManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// The session's connection (LISTEN run in the console).
    Console,
    /// The Notifications view's listener connection.
    Listener,
}

/// A notification arrived, or `dropped` of them were lost because the UI
/// couldn't keep up.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct NotificationReceived {
    pub session_id: Uuid,
    pub source: Source,
    pub notification: Option<NotificationPayload>,
    pub dropped: u32,
}

/// `Notification` for the event (events need Deserialize too).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPayload {
    pub channel: String,
    pub payload: String,
    pub process_id: i32,
    pub received_at: String,
}

impl From<Notification> for NotificationPayload {
    fn from(n: Notification) -> Self {
        Self {
            channel: n.channel,
            payload: n.payload,
            process_id: n.process_id,
            received_at: n.received_at.to_rfc3339(),
        }
    }
}

/// Sends `conn`'s notifications to the UI until the connection closes.
pub fn forward<R: tauri::Runtime>(
    app: &AppHandle<R>,
    session_id: Uuid,
    conn: &dyn Connection,
    source: Source,
) {
    let Some(mut rx) = conn.subscribe_notifications() else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let event = match rx.recv().await {
                Ok(n) => NotificationReceived {
                    session_id,
                    source,
                    notification: Some(n.into()),
                    dropped: 0,
                },
                Err(RecvError::Lagged(n)) => NotificationReceived {
                    session_id,
                    source,
                    notification: None,
                    dropped: n.min(u64::from(u32::MAX)) as u32,
                },
                Err(RecvError::Closed) => break,
            };
            if let Err(err) = event.emit(&app) {
                tracing::debug!(error = %err, "notification not delivered");
            }
        }
    });
}

struct Listener {
    conn: Box<dyn Connection>,
    channels: BTreeSet<String>,
}

/// Listener connections by session id.
#[derive(Default)]
pub struct Listeners {
    map: Mutex<HashMap<Uuid, Listener>>,
}

impl Listeners {
    /// Closes a session's listener, e.g. on disconnect.
    pub async fn close(&self, session_id: Uuid) {
        if let Some(listener) = self.map.lock().await.remove(&session_id) {
            listener.conn.close().await;
        }
    }
}

fn not_found(session_id: Uuid) -> DbError {
    DbError::new(
        ErrorKind::NotFound,
        format!("session {session_id} not found"),
    )
}

/// Opens the session's listener connection if it has none yet.
// The map entry is filled after awaits (connecting), so the entry API does not fit.
#[allow(clippy::map_entry)]
async fn ensure_listener<'a>(
    app: &AppHandle,
    sessions: &SessionManager,
    map: &'a mut HashMap<Uuid, Listener>,
    session_id: Uuid,
) -> Result<&'a mut Listener, DbError> {
    if let std::collections::hash_map::Entry::Vacant(e) = map.entry(session_id) {
        let session = sessions
            .get(session_id)
            .await
            .ok_or_else(|| not_found(session_id))?;
        let origin = session.origin.as_ref().ok_or_else(|| {
            DbError::new(
                ErrorKind::Unsupported,
                "this session can't open a listener connection",
            )
        })?;
        if !session.conn.capabilities().listen_notify {
            return Err(DbError::new(
                ErrorKind::Unsupported,
                "this database has no LISTEN / NOTIFY",
            ));
        }
        let conn = origin.driver.connect(&origin.config).await?;
        forward(app, session_id, &*conn, Source::Listener);
        e.insert(Listener {
            conn,
            channels: BTreeSet::new(),
        });
    }
    Ok(map.get_mut(&session_id).expect("inserted above"))
}

fn check_channel(channel: &str) -> Result<(), DbError> {
    if channel.trim().is_empty() || channel.len() > 63 {
        return Err(DbError::new(
            ErrorKind::Syntax,
            "A channel name must be 1 to 63 characters",
        ));
    }
    Ok(())
}

/// Starts listening on `channel` for the Notifications view. Returns the
/// channels listened on now.
#[tauri::command]
#[specta::specta]
pub async fn listen_channel(
    app: AppHandle,
    sessions: State<'_, SessionManager>,
    listeners: State<'_, Listeners>,
    session_id: Uuid,
    channel: String,
) -> Result<Vec<String>, DbError> {
    check_channel(&channel)?;
    let mut map = listeners.map.lock().await;
    let listener = ensure_listener(&app, &sessions, &mut map, session_id).await?;
    listener.conn.listen(&channel).await?;
    listener.channels.insert(channel);
    Ok(listener.channels.iter().cloned().collect())
}

/// Stops listening on `channel`; the listener connection closes with the last one.
#[tauri::command]
#[specta::specta]
pub async fn unlisten_channel(
    listeners: State<'_, Listeners>,
    session_id: Uuid,
    channel: String,
) -> Result<Vec<String>, DbError> {
    let mut map = listeners.map.lock().await;
    let Some(listener) = map.get_mut(&session_id) else {
        return Ok(Vec::new());
    };
    listener.conn.unlisten(&channel).await?;
    listener.channels.remove(&channel);
    let channels: Vec<String> = listener.channels.iter().cloned().collect();
    if channels.is_empty()
        && let Some(listener) = map.remove(&session_id)
    {
        listener.conn.close().await;
    }
    Ok(channels)
}

#[tauri::command]
#[specta::specta]
pub async fn list_listen_channels(
    listeners: State<'_, Listeners>,
    session_id: Uuid,
) -> Result<Vec<String>, DbError> {
    let map = listeners.map.lock().await;
    Ok(map
        .get(&session_id)
        .map(|l| l.channels.iter().cloned().collect())
        .unwrap_or_default())
}

/// NOTIFY `channel` with `payload`, on the listener connection (opened if
/// needed) so it doesn't wait behind a console query.
#[tauri::command]
#[specta::specta]
pub async fn send_notify(
    app: AppHandle,
    sessions: State<'_, SessionManager>,
    listeners: State<'_, Listeners>,
    session_id: Uuid,
    channel: String,
    payload: String,
) -> Result<(), DbError> {
    check_channel(&channel)?;
    if payload.len() >= 8000 {
        return Err(DbError::new(
            ErrorKind::Syntax,
            "A NOTIFY payload must be shorter than 8000 bytes",
        ));
    }
    let mut map = listeners.map.lock().await;
    let listener = ensure_listener(&app, &sessions, &mut map, session_id).await?;
    listener.conn.notify(&channel, &payload).await
}
