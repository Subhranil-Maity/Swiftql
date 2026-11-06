use async_trait::async_trait;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    capabilities::Capabilities,
    dbresults::DbResult,
    dialect::Dialect,
    error::{DbError, ErrorKind},
    introspec::{IntrospectScope, SchemaSnapshot},
    notice::Notice,
    notify::Notification,
    types::ColumnMeta,
    value::Value,
};
#[async_trait]
pub trait Connection: Send + Sync {
    fn dialect(&self) -> &dyn Dialect;
    fn server_version(&self) -> &ServerVersion;
    fn capabilities(&self) -> &Capabilities; // may depend on version
    fn tx_status(&self) -> TxStatus;

    async fn execute(&self, sql: &str) -> DbResult<ExecOutcome>;
    fn cancel_handle(&self) -> Box<dyn CancelHandle>;
    async fn introspect(&self, scope: IntrospectScope) -> DbResult<SchemaSnapshot>;
    async fn ping(&self) -> DbResult<()>;
    async fn close(self: Box<Self>);

    /// Notices received since the last call, oldest first. Statements on a
    /// connection run one at a time, so draining after each statement
    /// attributes its notices to it. Drivers without notices return nothing.
    fn take_notices(&self) -> Vec<Notice> {
        Vec::new()
    }

    /// Notifications this connection receives (after `listen`). Each call
    /// gives a new receiver; a slow one misses the oldest (`RecvError::Lagged`).
    /// None when the driver has no notifications (`Capabilities::listen_notify`).
    fn subscribe_notifications(&self) -> Option<tokio::sync::broadcast::Receiver<Notification>> {
        None
    }

    /// Starts receiving notifications on `channel` (a name, quoted by the driver).
    async fn listen(&self, channel: &str) -> DbResult<()> {
        let _ = channel;
        Err(no_notifications())
    }

    async fn unlisten(&self, channel: &str) -> DbResult<()> {
        let _ = channel;
        Err(no_notifications())
    }

    /// Sends a notification to everyone listening on `channel`.
    async fn notify(&self, channel: &str, payload: &str) -> DbResult<()> {
        let _ = (channel, payload);
        Err(no_notifications())
    }
}

fn no_notifications() -> DbError {
    DbError::new(
        ErrorKind::Unsupported,
        "this database has no LISTEN / NOTIFY",
    )
}

pub enum ExecOutcome {
    Rows {
        columns: Vec<ColumnMeta>,
        stream: Box<dyn RowStream>,
        /// Leading keyword, e.g. "SELECT", or "INSERT" for `INSERT ... RETURNING`;
        /// with the row count it makes the final tag (`Dialect::command_tag`).
        command: String,
    }, // streamed in batches
    Affected {
        rows: u64,
        command: String,
    }, // INSERT/UPDATE/DDL
}
#[async_trait]
pub trait RowStream: Send {
    /// Fetch up to `max` more rows. Returns None when nothing is left.
    async fn next_batch(&mut self, max: usize) -> DbResult<Option<RowBatch>>;

    /// Stop early and free the cursor on the server.
    async fn close(self: Box<Self>) -> DbResult<()>;
}
// core (driver output)
#[derive(Debug, Clone)]
pub struct RowBatch {
    pub rows: Vec<Vec<Value>>,
    pub has_more: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TxStatus {
    Idle, // not in a transaction
    InTransaction,
    Failed, // error inside a tx: everything fails until ROLLBACK
}
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ServerVersion {
    pub product: String, // "PostgreSQL", "MySQL", "MariaDB"
    pub major: u32,      // 16
    pub minor: u32,      // 4
    pub patch: u32,      // 0 (Postgres 10+ has only major.minor)
    pub raw: String,     // full string from the server, for display
}

impl ServerVersion {
    pub fn at_least(&self, major: u32, minor: u32) -> bool {
        (self.major, self.minor) >= (major, minor)
    }
}
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RowEvent {
    pub query_id: Uuid,
    pub rows: Vec<Vec<Value>>, // each inner Vec is one row, in column order
    pub offset: u64,           // index of the first row in this batch
    pub has_more: bool,        // false = this was the last batch
}

#[async_trait]
pub trait CancelHandle: Send + Sync {
    async fn cancel(&self) -> DbResult<()>;
}
