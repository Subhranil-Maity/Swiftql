//! Running console SQL: statements run one by one on a session. Their rows
//! are read into a held result (see `results`), and the first page, notices
//! and outcomes stream to the UI over a channel.

use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use config_store::settings::{QuerySettings, TransactionMode};
use driver_core::{
    connection::{ExecOutcome, RowStream, TxStatus},
    error::{DbError, ErrorKind},
    notice::Notice,
    types::ColumnMeta,
    value::Value,
};
use serde::Serialize;
use specta::Type;
use tauri::{State, ipc::Channel};
use uuid::Uuid;

use crate::{
    config::Config,
    results::{self, ResultState, SharedBuffer},
    session::{Session, SessionManager, TIMEOUT_UNKNOWN},
};

/// Rows read from the driver at a time.
const BATCH_SIZE: usize = 500;

/// How often a `progress` event is sent while rows are read.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// SQLSTATE for both a user cancel and `statement_timeout`.
const QUERY_CANCELED: &str = "57014";

/// What happens during a run, in order. `index` is the statement's position
/// in the list passed to `run_statements`.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum RunEvent {
    StatementStarted {
        index: u32,
    },
    /// Before any `rows`; only for statements that return rows. The rows are
    /// held under `resultId` for `fetch_rows`.
    #[serde(rename_all = "camelCase")]
    Columns {
        index: u32,
        columns: Vec<ColumnMeta>,
        result_id: Uuid,
    },
    /// The first rows of a result (up to `query.pageSize`), so they show at once.
    Rows {
        index: u32,
        rows: Vec<Vec<Value>>,
    },
    /// Rows held so far, while they're read.
    Progress {
        index: u32,
        buffered: u64,
    },
    Notice {
        index: u32,
        notice: Notice,
    },
    #[serde(rename_all = "camelCase")]
    StatementFinished {
        index: u32,
        /// e.g. "INSERT 0 3", "CREATE TABLE", "SELECT 1000".
        tag: String,
        /// Rows held or changed.
        rows: u64,
        elapsed_ms: f64,
        /// For statements that return rows: whether every row was read.
        result: Option<ResultState>,
    },
    #[serde(rename_all = "camelCase")]
    StatementFailed {
        index: u32,
        error: DbError,
        elapsed_ms: f64,
    },
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    /// Statements that finished without error.
    pub executed: u32,
    /// The statement that failed; the ones after it were skipped.
    pub failed_at: Option<u32>,
    /// Stopped by `cancel_run`.
    pub cancelled: bool,
    pub elapsed_ms: f64,
    /// Transaction status after the run.
    pub tx_status: TxStatus,
}

/// How much of a result is held, from the settings.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Rows sent with the `rows` event.
    pub first_page: usize,
    pub max_rows: u64,
    pub max_bytes: u64,
}

impl Limits {
    pub fn from_settings(query: &QuerySettings) -> Self {
        Self {
            first_page: query.page_size as usize,
            max_rows: match query.max_rows_held {
                0 => u64::MAX,
                n => u64::from(n),
            },
            max_bytes: u64::from(query.max_result_mb) * 1024 * 1024,
        }
    }
}

fn ms(elapsed: Duration) -> f64 {
    elapsed.as_secs_f64() * 1000.0
}

fn not_found(session_id: Uuid) -> DbError {
    DbError::new(
        ErrorKind::NotFound,
        format!("session {session_id} not found"),
    )
}

fn busy() -> DbError {
    DbError::new(
        ErrorKind::Busy,
        "A query is already running in this session",
    )
}

/// Runs `statements` in order on a session and stops at the first error.
/// Rows are held up to the `query.maxRowsHeld` / `query.maxResultMb` caps;
/// past them the query is cancelled on the server.
///
/// Fails with `NotFound` for an unknown session and `Busy` while another run
/// holds the session. Statement errors are `statementFailed` events, not
/// errors of the command.
#[tauri::command]
#[specta::specta]
pub async fn run_statements(
    config: State<'_, Config>,
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    statements: Vec<String>,
    on_event: Channel<RunEvent>,
) -> Result<RunSummary, DbError> {
    let query = config.settings().get().query;
    run_with(&sessions, session_id, statements, &query, on_event).await
}

/// `run_statements` without the managed settings, for tests.
pub async fn run_with(
    sessions: &SessionManager,
    session_id: Uuid,
    statements: Vec<String>,
    query: &QuerySettings,
    on_event: Channel<RunEvent>,
) -> Result<RunSummary, DbError> {
    let session = sessions
        .get(session_id)
        .await
        .ok_or_else(|| not_found(session_id))?;
    let _running = session.run_lock.try_lock().map_err(|_| busy())?;
    session.cancelled.store(false, Ordering::SeqCst);

    let emit = |event: RunEvent| {
        // The window may be gone; the run still finishes so the session is left clean.
        if let Err(err) = on_event.send(event) {
            tracing::debug!(error = %err, "run event not delivered");
        }
    };
    prepare(&session, query).await;
    Ok(run(&session, &statements, Limits::from_settings(query), &emit).await)
}

/// Applies the query timeout and, in manual transaction mode, opens a
/// transaction. Only while idle: inside a transaction a SET would be undone
/// by a rollback, and in a failed one it can't run at all.
async fn prepare(session: &Session, query: &QuerySettings) {
    if session.conn.tx_status() != TxStatus::Idle {
        return;
    }
    let wanted = query.query_timeout_secs;
    if session.applied_timeout.load(Ordering::SeqCst) != wanted {
        let sql = format!("SET statement_timeout = {}", u64::from(wanted) * 1000);
        match session.conn.execute(&sql).await {
            Ok(_) => session.applied_timeout.store(wanted, Ordering::SeqCst),
            Err(err) => {
                session
                    .applied_timeout
                    .store(TIMEOUT_UNKNOWN, Ordering::SeqCst);
                tracing::warn!(error = %err, "couldn't set statement_timeout");
            }
        }
    }
    if query.transaction_mode == TransactionMode::Manual
        && let Err(err) = session.conn.execute("BEGIN").await
    {
        tracing::warn!(error = %err, "couldn't open a transaction");
    }
}

async fn run(
    session: &Session,
    statements: &[String],
    limits: Limits,
    emit: &impl Fn(RunEvent),
) -> RunSummary {
    let started = Instant::now();
    // Notices left over from outside the console (introspection, ...) aren't this run's.
    session.conn.take_notices();

    let mut executed = 0;
    let mut failed_at = None;
    let mut cancelled = false;
    for (index, sql) in statements.iter().enumerate() {
        let index = index as u32;
        if session.cancelled.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }
        emit(RunEvent::StatementStarted { index });
        let statement_started = Instant::now();
        *session.in_flight.lock().await = true;
        let result = run_one(session, index, sql, limits, emit).await;
        *session.in_flight.lock().await = false;
        drain_notices(session, index, emit);
        let elapsed_ms = ms(statement_started.elapsed());
        match result {
            Ok((tag, rows, result)) => {
                executed += 1;
                emit(RunEvent::StatementFinished {
                    index,
                    tag,
                    rows,
                    elapsed_ms,
                    result,
                });
                if result == Some(ResultState::Cancelled) {
                    // Stop kept the rows read so far; the rest of the script is skipped.
                    cancelled = true;
                    break;
                }
            }
            Err(error) => {
                let error = classify_cancel(error, session);
                cancelled = error.kind == ErrorKind::Cancelled;
                failed_at = Some(index);
                emit(RunEvent::StatementFailed {
                    index,
                    error,
                    elapsed_ms,
                });
                break;
            }
        }
    }

    RunSummary {
        executed,
        failed_at,
        cancelled,
        elapsed_ms: ms(started.elapsed()),
        tx_status: session.conn.tx_status(),
    }
}

/// A user cancel and `statement_timeout` share SQLSTATE 57014; tell them
/// apart by whether this session sent a cancel, not by the (localized) message.
fn classify_cancel(mut error: DbError, session: &Session) -> DbError {
    if error.code.as_deref() == Some(QUERY_CANCELED) {
        if session.cancelled.load(Ordering::SeqCst) {
            error.kind = ErrorKind::Cancelled;
        } else {
            error.kind = ErrorKind::Timeout;
            error.message = "Query timed out (statement_timeout)".into();
        }
    }
    error
}

/// One statement: (tag, rows held or changed, result state for row results).
async fn run_one(
    session: &Session,
    index: u32,
    sql: &str,
    limits: Limits,
    emit: &impl Fn(RunEvent),
) -> Result<(String, u64, Option<ResultState>), DbError> {
    let dialect = session.conn.dialect();
    match session.conn.execute(sql).await? {
        ExecOutcome::Affected { rows, command } => {
            Ok((dialect.command_tag(&command, rows), rows, None))
        }
        ExecOutcome::Rows {
            columns,
            stream,
            command,
        } => {
            let (result_id, buffer) = session.results.insert(columns.clone());
            emit(RunEvent::Columns {
                index,
                columns,
                result_id,
            });
            let (rows, state) = read_rows(session, index, stream, &buffer, limits, emit).await?;
            Ok((dialect.command_tag(&command, rows), rows, Some(state)))
        }
    }
}

/// Reads rows into `buffer` until the stream ends, a cap is reached or the
/// user stops it. Sends the first page and progress on the way.
async fn read_rows(
    session: &Session,
    index: u32,
    mut stream: Box<dyn RowStream>,
    buffer: &SharedBuffer,
    limits: Limits,
    emit: &impl Fn(RunEvent),
) -> Result<(u64, ResultState), DbError> {
    let mut held = 0u64;
    let mut sent = 0usize;
    let mut last_progress = Instant::now();
    let set_state = |state| results::write(buffer).state = state;

    loop {
        if session.cancelled.load(Ordering::SeqCst) {
            // Rows already buffered would otherwise keep coming after Stop.
            let _ = stream.close().await;
            set_state(ResultState::Cancelled);
            return Ok((held, ResultState::Cancelled));
        }
        let batch = match stream.next_batch(BATCH_SIZE).await {
            Ok(Some(batch)) => batch,
            Ok(None) => break,
            Err(err) => {
                let _ = stream.close().await;
                if session.cancelled.load(Ordering::SeqCst) {
                    set_state(ResultState::Cancelled);
                    return Ok((held, ResultState::Cancelled));
                }
                set_state(ResultState::Failed);
                return Err(err);
            }
        };
        held += batch.rows.len() as u64;
        if sent < limits.first_page {
            let take = (limits.first_page - sent).min(batch.rows.len());
            sent += take;
            emit(RunEvent::Rows {
                index,
                rows: batch.rows[..take].to_vec(),
            });
        }
        let has_more = batch.has_more;
        let bytes = {
            let mut b = results::write(buffer);
            b.push_rows(batch.rows);
            b.est_bytes
        };
        drain_notices(session, index, emit);
        if last_progress.elapsed() >= PROGRESS_EVERY {
            last_progress = Instant::now();
            emit(RunEvent::Progress {
                index,
                buffered: held,
            });
        }
        if !has_more {
            break;
        }
        if held >= limits.max_rows || bytes >= limits.max_bytes {
            // Cancels the query and reads up to its end, so the session is free.
            stream.close().await?;
            set_state(ResultState::Truncated);
            return Ok((held, ResultState::Truncated));
        }
    }
    stream.close().await?;
    set_state(ResultState::Complete);
    Ok((held, ResultState::Complete))
}

fn drain_notices(session: &Session, index: u32, emit: &impl Fn(RunEvent)) {
    for notice in session.conn.take_notices() {
        emit(RunEvent::Notice { index, notice });
    }
}

/// Stops the session's run, if there is one. The running statement fails
/// with `Cancelled` (or, while rows are read, keeps the rows read so far)
/// and the rest are not started.
#[tauri::command]
#[specta::specta]
pub async fn cancel_run(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
) -> Result<(), DbError> {
    let session = sessions
        .get(session_id)
        .await
        .ok_or_else(|| not_found(session_id))?;
    // Not checked against run_lock: taking it, even briefly, could make a run
    // that is just starting fail with Busy.
    session.cancelled.store(true, Ordering::SeqCst);
    // Only cancel on the server while a statement is in flight; the run
    // can't mark it finished while this lock is held.
    let in_flight = session.in_flight.lock().await;
    if *in_flight {
        // TODO: the server can still finish the statement between our check
        // and the cancel arriving; the cancel is then ignored, or (rarely) hits
        // the run's next statement, which then fails as Cancelled.
        session.conn.cancel_handle().cancel().await?;
    }
    Ok(())
}

/// Ends the open transaction with COMMIT or ROLLBACK. Fails with `Busy`
/// while a run holds the session.
#[tauri::command]
#[specta::specta]
pub async fn end_transaction(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    commit: bool,
) -> Result<TxStatus, DbError> {
    let session = sessions
        .get(session_id)
        .await
        .ok_or_else(|| not_found(session_id))?;
    let _running = session.run_lock.try_lock().map_err(|_| busy())?;
    let sql = if commit { "COMMIT" } else { "ROLLBACK" };
    session.conn.execute(sql).await?;
    Ok(session.conn.tx_status())
}

/// Runs against a real server:
/// SWIFTQL_PG_URL=postgres://user:pass@host/db cargo test --manifest-path src-tauri/Cargo.toml -- --ignored
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use driver_core::driver::Driver;
    use driver_postgres::PgDriver;
    use serde_json::Value as Json;
    use tauri::{Manager, ipc::InvokeResponseBody, test::MockRuntime};

    use super::*;

    fn app() -> tauri::App<MockRuntime> {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        app.manage(SessionManager::default());
        app
    }

    async fn open_session(app: &tauri::App<MockRuntime>) -> Uuid {
        let url = std::env::var("SWIFTQL_PG_URL").expect("set SWIFTQL_PG_URL");
        let conn = PgDriver
            .connect(&PgDriver.parse_url(&url).unwrap())
            .await
            .unwrap();
        let session = Session::new(Uuid::new_v4(), "postgres", conn);
        app.state::<SessionManager>().insert(session).await.id
    }

    /// A channel that records every event as JSON, as the frontend sees it.
    fn recorder() -> (Channel<RunEvent>, Arc<Mutex<Vec<Json>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let channel = Channel::new(move |body| {
            if let InvokeResponseBody::Json(json) = body {
                sink.lock()
                    .unwrap()
                    .push(serde_json::from_str(&json).unwrap());
            }
            Ok(())
        });
        (channel, events)
    }

    fn sql(statements: &[&str]) -> Vec<String> {
        statements.iter().map(|s| s.to_string()).collect()
    }

    /// Query settings holding at most `max_rows` rows, sending them all as the first page.
    fn settings(max_rows: u32) -> QuerySettings {
        QuerySettings {
            page_size: max_rows.max(50),
            max_rows_held: max_rows,
            ..QuerySettings::default()
        }
    }

    /// "statementStarted 0", "columns 0", ... for comparing the order.
    fn names(events: &[Json]) -> Vec<String> {
        events
            .iter()
            .map(|e| format!("{} {}", e["event"].as_str().unwrap(), e["index"]))
            .collect()
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs SWIFTQL_PG_URL"]
    async fn events_in_order() {
        let app = app();
        let id = open_session(&app).await;
        let (channel, events) = recorder();
        let statements = sql(&[
            "CREATE TEMP TABLE t (a int)",
            "DO $$ BEGIN RAISE NOTICE 'hi'; END $$",
            "INSERT INTO t VALUES (1), (2) RETURNING a",
        ]);
        let summary = run_with(&app.state(), id, statements, &settings(1000), channel)
            .await
            .unwrap();
        assert_eq!((summary.executed, summary.failed_at), (3, None));
        assert!(!summary.cancelled);

        let events = events.lock().unwrap();
        assert_eq!(
            names(&events),
            [
                "statementStarted 0",
                "statementFinished 0",
                "statementStarted 1",
                "notice 1",
                "statementFinished 1",
                "statementStarted 2",
                "columns 2",
                "rows 2",
                "statementFinished 2",
            ]
        );
        assert_eq!(events[1]["tag"], "CREATE TABLE");
        assert_eq!(events[3]["notice"]["message"], "hi");
        assert_eq!(events[3]["notice"]["severity"], "notice");
        assert_eq!(events[6]["columns"][0]["name"], "a");
        assert_eq!(events[7]["rows"].as_array().unwrap().len(), 2);
        assert_eq!(events[8]["tag"], "INSERT 0 2");
        assert!(events[8]["elapsedMs"].as_f64().is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs SWIFTQL_PG_URL"]
    async fn truncates_at_row_limit() {
        let app = app();
        let id = open_session(&app).await;
        let (channel, events) = recorder();
        let statements = sql(&["SELECT generate_series(1, 1500)", "SELECT 42"]);
        let summary = run_with(&app.state(), id, statements, &settings(1000), channel)
            .await
            .unwrap();
        assert_eq!(summary.executed, 2); // the session is usable after the cancel

        let events = events.lock().unwrap();
        let sent: usize = events
            .iter()
            .filter(|e| e["event"] == "rows" && e["index"] == 0)
            .map(|e| e["rows"].as_array().unwrap().len())
            .sum();
        assert_eq!(sent, 1000);
        let finished: Vec<_> = events
            .iter()
            .filter(|e| e["event"] == "statementFinished")
            .collect();
        assert_eq!(finished[0]["result"], "truncated");
        assert_eq!(finished[0]["tag"], "SELECT 1000");
        assert_eq!(finished[1]["result"], "complete");
        assert_eq!(finished[1]["tag"], "SELECT 1");
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs SWIFTQL_PG_URL"]
    async fn stops_at_first_error() {
        let app = app();
        let id = open_session(&app).await;
        let (channel, events) = recorder();
        let statements = sql(&["SELECT 1", "SELEC 1", "SELECT 2"]);
        let summary = run_with(&app.state(), id, statements, &settings(1000), channel)
            .await
            .unwrap();
        assert_eq!((summary.executed, summary.failed_at), (1, Some(1)));
        assert!(!summary.cancelled);

        let events = events.lock().unwrap();
        let failed = events.last().unwrap();
        assert_eq!(failed["event"], "statementFailed");
        assert_eq!(failed["error"]["code"], "42601");
        assert_eq!(failed["error"]["position"], 0);
        assert!(!names(&events).iter().any(|n| n.ends_with(" 2")));
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs SWIFTQL_PG_URL"]
    async fn busy_then_cancel() {
        let app = app();
        let id = open_session(&app).await;
        let (channel, started) = recorder();
        let handle = app.handle().clone();
        let long = tokio::spawn(async move {
            let statements = sql(&["SELECT pg_sleep(30)", "SELECT 1"]);
            run_with(&handle.state(), id, statements, &settings(1000), channel).await
        });
        // Wait until the first run holds the lock and has sent its statement.
        while started.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let (channel, _) = recorder();
        let err = run_with(
            &app.state(),
            id,
            sql(&["SELECT 1"]),
            &settings(1000),
            channel,
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind, ErrorKind::Busy);

        // Let the statement reach the server before cancelling it.
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancel_run(app.state(), id).await.unwrap();
        let summary = long.await.unwrap().unwrap();
        assert!(summary.cancelled);
        assert_eq!((summary.executed, summary.failed_at), (0, Some(0)));
    }

    #[tokio::test]
    async fn unknown_session() {
        let app = app();
        let (channel, _) = recorder();
        let err = run_with(
            &app.state(),
            Uuid::new_v4(),
            sql(&["SELECT 1"]),
            &settings(10),
            channel,
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind, ErrorKind::NotFound);
        assert_eq!(
            cancel_run(app.state(), Uuid::new_v4())
                .await
                .unwrap_err()
                .kind,
            ErrorKind::NotFound
        );
    }
}
