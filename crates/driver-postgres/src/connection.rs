use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use driver_core::{
    capabilities::Capabilities,
    connection::{CancelHandle, Connection, ExecOutcome, ServerVersion, TxStatus},
    dbresults::DbResult,
    dialect::Dialect,
    error::{DbError, ErrorKind},
    introspec::{IntrospectScope, SchemaSnapshot},
};
use tokio_postgres::{
    Client, NoTls, Socket,
    tls::{MakeTlsConnect, TlsConnect},
    types::ToSql,
};

use crate::{
    cancel::PgCancel,
    capabilities,
    dialect::PgDialect,
    error::map_err,
    introspect, sql,
    stream::PgRowStream,
    tls::PgTls,
    tx::{self, TxCommand, TxState},
    types::column_meta,
};

/// Oldest supported server (server_version_num of PostgreSQL 10.0).
const MIN_VERSION_NUM: i32 = 100_000;

pub struct PgConnection {
    pub(crate) client: Client,
    pub(crate) dialect: PgDialect,
    pub(crate) version: ServerVersion,
    pub(crate) capabilities: Capabilities, // version-dependent
    pub(crate) tls: PgTls,                 // reused for cancel requests
    /// Why the background connection task stopped, if it failed.
    pub(crate) closed: Arc<Mutex<Option<DbError>>>,
    pub(crate) tx: TxState,
}

impl PgConnection {
    /// Connect, start the background connection task and read the server version.
    pub async fn open(config: &tokio_postgres::Config, tls: PgTls) -> DbResult<Self> {
        let (client, closed) = match &tls {
            PgTls::None => spawn_connection(config, NoTls).await?,
            PgTls::Native(connector) => spawn_connection(config, connector.clone()).await?,
        };

        let version = server_version(&client).await?;
        Ok(Self {
            client,
            dialect: PgDialect,
            capabilities: capabilities::for_version(&version),
            version,
            tls,
            closed,
            tx: Arc::new(Mutex::new(TxStatus::Idle)),
        })
    }

    /// Fail fast if the connection already died, with the real cause.
    fn check_open(&self) -> DbResult<()> {
        match self.closed_error() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// A call that failed because the connection died reports why it died.
    fn with_cause<T>(&self, result: DbResult<T>) -> DbResult<T> {
        result.map_err(|err| self.closed_error().unwrap_or(err))
    }

    /// Server errors abort an open transaction. Errors made on our side
    /// (no SQLSTATE, e.g. Unsupported) never reached the server and don't.
    fn track<T>(&self, command: TxCommand, result: &DbResult<T>) {
        match result {
            Ok(_) => tx::after_success(&self.tx, command),
            Err(err) if err.code.is_some() => tx::after_error(&self.tx, command),
            Err(_) => {}
        }
    }

    async fn run(&self, sql: &str) -> DbResult<ExecOutcome> {
        let statement = self.client.prepare(sql).await.map_err(|err| {
            let err = map_err(err);
            if is_multi_statement(&err) {
                DbError {
                    kind: ErrorKind::Unsupported,
                    message:
                        "multiple statements are not supported yet; run one statement at a time"
                            .into(),
                    ..err
                }
            } else {
                err
            }
        })?;
        if !statement.params().is_empty() {
            return Err(DbError::new(
                ErrorKind::Unsupported,
                "TODO: query parameters ($1, $2, ...) are not supported yet",
            ));
        }

        if statement.columns().is_empty() {
            let rows = self
                .client
                .execute(&statement, &[])
                .await
                .map_err(map_err)?;
            return Ok(ExecOutcome::Affected {
                rows,
                command: sql::command_tag(sql),
            });
        }

        let columns = statement.columns().iter().map(column_meta).collect();
        let no_params = std::iter::empty::<&(dyn ToSql + Sync)>();
        let rows = self
            .client
            .query_raw(&statement, no_params)
            .await
            .map_err(map_err)?;
        let cancel = PgCancel {
            token: self.client.cancel_token(),
            tls: self.tls.clone(),
        };
        Ok(ExecOutcome::Rows {
            columns,
            stream: Box::new(PgRowStream::new(rows, cancel, Arc::clone(&self.tx))),
        })
    }

    fn closed_error(&self) -> Option<DbError> {
        self.closed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

/// tokio-postgres hands back the socket as a future that must be polled for the
/// client to work, so it runs on its own task. Its error is kept for later calls.
async fn spawn_connection<T>(
    config: &tokio_postgres::Config,
    tls: T,
) -> DbResult<(Client, Arc<Mutex<Option<DbError>>>)>
where
    T: MakeTlsConnect<Socket>,
    T::Stream: Send + 'static,
    T::TlsConnect: Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    let (client, connection) = config.connect(tls).await.map_err(map_err)?;

    let closed = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&closed);
    tokio::spawn(async move {
        if let Err(err) = connection.await {
            *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(map_err(err));
        }
    });
    Ok((client, closed))
}

/// The server refuses to prepare "SELECT 1; SELECT 2".
/// TODO: the message is localized (lc_messages); only English is recognised.
fn is_multi_statement(err: &DbError) -> bool {
    err.code.as_deref() == Some("42601") && err.message.contains("multiple commands")
}

async fn server_version(client: &Client) -> DbResult<ServerVersion> {
    let row = client
        .query_one(
            "SELECT current_setting('server_version_num')::int, current_setting('server_version')",
            &[],
        )
        .await
        .map_err(map_err)?;
    let num: i32 = row.get(0);
    let raw: String = row.get(1);

    if num < MIN_VERSION_NUM {
        return Err(DbError::new(
            ErrorKind::Unsupported,
            format!("PostgreSQL {raw} is not supported; version 10 or newer is required"),
        ));
    }
    // PG 10+: major * 10000 + minor, e.g. 160004 = 16.4
    let num = num as u32;
    Ok(ServerVersion {
        product: "PostgreSQL".into(),
        major: num / 10_000,
        minor: num % 10_000,
        patch: 0,
        raw,
    })
}

#[async_trait]
impl Connection for PgConnection {
    fn dialect(&self) -> &dyn Dialect {
        &self.dialect
    }

    fn server_version(&self) -> &ServerVersion {
        &self.version
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn tx_status(&self) -> TxStatus {
        tx::get(&self.tx)
    }

    /// One statement. Returns rows if it has columns (SELECT, ... RETURNING),
    /// otherwise the affected row count.
    async fn execute(&self, sql: &str) -> DbResult<ExecOutcome> {
        self.check_open()?;
        let result = self.run(sql).await;
        self.track(TxCommand::of(sql), &result);
        self.with_cause(result)
    }

    fn cancel_handle(&self) -> Box<dyn CancelHandle> {
        Box::new(PgCancel {
            token: self.client.cancel_token(),
            tls: self.tls.clone(),
        })
    }

    async fn introspect(&self, scope: IntrospectScope) -> DbResult<SchemaSnapshot> {
        self.check_open()?;
        let result = introspect::introspect(&self.client, &self.version, scope).await;
        self.track(TxCommand::Other, &result); // runs inside the user's open transaction
        self.with_cause(result)
    }

    async fn ping(&self) -> DbResult<()> {
        self.check_open()?;
        let result = self.client.simple_query("SELECT 1").await.map_err(map_err);
        self.track(TxCommand::Other, &result);
        self.with_cause(result).map(|_| ())
    }

    async fn close(self: Box<Self>) {
        // Dropping the client sends Terminate; the connection task then ends by itself.
    }
}
