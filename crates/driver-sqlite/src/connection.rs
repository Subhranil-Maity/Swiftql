use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};

use async_trait::async_trait;
use driver_core::{
    capabilities::Capabilities,
    connection::{CancelHandle, Connection, ExecOutcome, ServerVersion, TxStatus},
    dbresults::DbResult,
    dialect::Dialect,
    introspec::{IntrospectScope, SchemaSnapshot},
};
use tokio::sync::oneshot;

use crate::{
    capabilities,
    dialect::SqliteDialect,
    error::map_err,
    introspect,
    stream::SqliteRowStream,
    worker::{self, Job, Shared, Started, gone},
};

pub struct SqliteConnection {
    jobs: mpsc::Sender<Job>,
    interrupt: Arc<rusqlite::InterruptHandle>,
    shared: Arc<Shared>,
    next_stream: AtomicU64,
    dialect: SqliteDialect,
    version: ServerVersion,
    capabilities: Capabilities,
}

impl SqliteConnection {
    pub async fn open(path: PathBuf, read_only: bool) -> DbResult<Self> {
        let opened = worker::open(path, read_only).await?;
        let version = server_version();
        Ok(Self {
            jobs: opened.jobs,
            interrupt: Arc::new(opened.interrupt),
            shared: opened.shared,
            next_stream: AtomicU64::new(1),
            dialect: SqliteDialect,
            capabilities: capabilities::for_version(&version),
            version,
        })
    }

    /// Runs `f` on the worker thread with the connection.
    async fn call<T, F>(&self, f: F) -> DbResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&rusqlite::Connection) -> DbResult<T> + Send + 'static,
    {
        let (reply, rx) = oneshot::channel();
        let shared = Arc::clone(&self.shared);
        self.jobs
            .send(Job::Call(Box::new(move |conn| {
                let result = f(conn);
                worker::track_tx(conn, &shared);
                let _ = reply.send(result);
            })))
            .map_err(|_| gone())?;
        rx.await.map_err(|_| gone())?
    }
}

/// The SQLite library compiled into the app.
fn server_version() -> ServerVersion {
    let raw = rusqlite::version().to_owned();
    let mut parts = raw.split('.').map(|p| p.parse().unwrap_or(0));
    ServerVersion {
        product: "SQLite".into(),
        major: parts.next().unwrap_or(0),
        minor: parts.next().unwrap_or(0),
        patch: parts.next().unwrap_or(0),
        raw,
    }
}

struct SqliteCancel(Arc<rusqlite::InterruptHandle>);

#[async_trait]
impl CancelHandle for SqliteCancel {
    async fn cancel(&self) -> DbResult<()> {
        // A no-op when nothing is running.
        self.0.interrupt();
        Ok(())
    }
}

#[async_trait]
impl Connection for SqliteConnection {
    fn dialect(&self) -> &dyn Dialect {
        &self.dialect
    }

    fn server_version(&self) -> &ServerVersion {
        &self.version
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    /// SQLite has no failed-transaction state: after an error the
    /// transaction stays open and usable.
    fn tx_status(&self) -> TxStatus {
        if self.shared.in_transaction.load(Ordering::SeqCst) {
            TxStatus::InTransaction
        } else {
            TxStatus::Idle
        }
    }

    async fn execute(&self, sql: &str) -> DbResult<ExecOutcome> {
        let stream_id = self.next_stream.fetch_add(1, Ordering::SeqCst);
        let (reply, rx) = oneshot::channel();
        self.jobs
            .send(Job::Execute {
                sql: sql.to_owned(),
                stream_id,
                reply,
            })
            .map_err(|_| gone())?;
        match rx.await.map_err(|_| gone())?? {
            Started::Affected { rows, command } => Ok(ExecOutcome::Affected { rows, command }),
            Started::Rows { columns, command } => Ok(ExecOutcome::Rows {
                columns,
                command,
                stream: Box::new(SqliteRowStream {
                    id: stream_id,
                    jobs: self.jobs.clone(),
                    done: false,
                }),
            }),
        }
    }

    fn cancel_handle(&self) -> Box<dyn CancelHandle> {
        Box::new(SqliteCancel(Arc::clone(&self.interrupt)))
    }

    async fn set_query_timeout(&self, secs: u32) -> DbResult<()> {
        self.shared.timeout_secs.store(secs, Ordering::SeqCst);
        Ok(())
    }

    async fn introspect(&self, scope: IntrospectScope) -> DbResult<SchemaSnapshot> {
        self.call(move |conn| introspect::introspect(conn, scope))
            .await
    }

    async fn ping(&self) -> DbResult<()> {
        self.call(|conn| {
            conn.query_row("SELECT 1", [], |_| Ok(()))
                .map_err(|e| map_err(e, false))
        })
        .await
    }

    async fn close(self: Box<Self>) {
        // Dropping the last job sender ends the worker thread, which closes
        // the database. Open row streams hold senders too and end with it.
    }
}
