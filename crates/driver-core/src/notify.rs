//! LISTEN / NOTIFY style notifications from the server.

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub channel: String,
    pub payload: String,
    /// Server process that sent it.
    pub process_id: i32,
    pub received_at: DateTime<Utc>,
}
