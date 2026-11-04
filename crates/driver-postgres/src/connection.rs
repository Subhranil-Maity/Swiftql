use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use driver_core::{
    capabilities::Capabilities,
    connection::{CancelHandle, Connection, ExecOutcome, ServerVersion, TxStatus},
    dbresults::DbResult,
    dialect::Dialect,
    error::{DbError, ErrorKind},
    introspec::{IntrospectScope, SchemaSnapshot},
    notice::{Notice, NoticeSeverity},
};
use tokio_postgres::{
    AsyncMessage, Client, NoTls, Socket,
    error::Severity,
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

/// Notices kept until someone reads them; older ones are dropped first.
const MAX_NOTICES: usize = 1000;

/// Filled by the connection task, drained by `take_notices`.
pub(crate) type NoticeQueue = Arc<Mutex<VecDeque<Notice>>>;

pub struct PgConnection {
    pub(crate) client: Client,
    pub(crate) dialect: PgDialect,
    pub(crate) version: ServerVersion,
    pub(crate) capabilities: Capabilities, // version-dependent
    pub(crate) tls: PgTls,                 // reused for cancel requests
    /// Why the background connection task stopped, if it failed.
    pub(crate) closed: Arc<Mutex<Option<DbError>>>,
    pub(crate) notices: NoticeQueue,
    pub(crate) tx: TxState,
}

impl PgConnection {
    /// Connect, start the background connection task and read the server version.
    pub async fn open(config: &tokio_postgres::Config, tls: PgTls) -> DbResult<Self> {
        let notices: NoticeQueue = Arc::default();
        let (client, closed) = match &tls {
            PgTls::None => spawn_connection(config, NoTls, Arc::clone(&notices)).await?,
            PgTls::Native(connector) => {
                spawn_connection(config, connector.clone(), Arc::clone(&notices)).await?
            }
        };

        let version = server_version(&client).await?;
        Ok(Self {
            client,
            dialect: PgDialect,
            capabilities: capabilities::for_version(&version),
            version,
            tls,
            closed,
            notices,
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
            command: sql::command_tag(sql),
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
/// client to work, so it runs on its own task. Notices it reads go to `notices`;
/// its error is kept for later calls.
async fn spawn_connection<T>(
    config: &tokio_postgres::Config,
    tls: T,
    notices: NoticeQueue,
) -> DbResult<(Client, Arc<Mutex<Option<DbError>>>)>
where
    T: MakeTlsConnect<Socket>,
    T::Stream: Send + 'static,
    T::TlsConnect: Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    let (client, mut connection) = config.connect(tls).await.map_err(map_err)?;

    let closed = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&closed);
    tokio::spawn(async move {
        // Messages arrive in socket order, so a statement's notices are queued
        // before the client sees that statement complete.
        loop {
            match futures::future::poll_fn(|cx| connection.poll_message(cx)).await {
                Some(Ok(AsyncMessage::Notice(notice))) => push_notice(&notices, to_notice(&notice)),
                Some(Ok(_)) => {} // TODO: LISTEN/NOTIFY
                Some(Err(err)) => {
                    *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(map_err(err));
                    break;
                }
                None => break, // client dropped
            }
        }
    });
    Ok((client, closed))
}

fn push_notice(queue: &NoticeQueue, notice: Notice) {
    let mut queue = queue.lock().unwrap_or_else(|e| e.into_inner());
    if queue.len() >= MAX_NOTICES {
        queue.pop_front();
    }
    queue.push_back(notice);
}

fn to_notice(db: &tokio_postgres::error::DbError) -> Notice {
    // parsed_severity reads the untranslated field (PG 9.6+); older servers
    // only send the localized one.
    let severity = match db.parsed_severity() {
        Some(Severity::Warning) => NoticeSeverity::Warning,
        Some(Severity::Info) => NoticeSeverity::Info,
        Some(Severity::Log) => NoticeSeverity::Log,
        Some(Severity::Debug) => NoticeSeverity::Debug,
        Some(_) => NoticeSeverity::Notice,
        None => match db.severity() {
            "WARNING" => NoticeSeverity::Warning,
            "INFO" => NoticeSeverity::Info,
            "LOG" => NoticeSeverity::Log,
            s if s.starts_with("DEBUG") => NoticeSeverity::Debug,
            _ => NoticeSeverity::Notice,
        },
    };
    Notice {
        severity,
        message: db.message().to_owned(),
        code: Some(db.code().code().to_owned()),
        detail: db.detail().map(str::to_owned),
        hint: db.hint().map(str::to_owned),
    }
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

    fn take_notices(&self) -> Vec<Notice> {
        let mut queue = self.notices.lock().unwrap_or_else(|e| e.into_inner());
        queue.drain(..).collect()
    }
}
