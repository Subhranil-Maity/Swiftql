//! Running console SQL: statements run one by one on a session, and their
//! rows, notices and outcomes stream to the UI over a channel.

use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

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

use crate::session::{Session, SessionManager};

/// Rows per `rows` event.
const BATCH_SIZE: usize = 200;

/// What happens during a run, in order. `index` is the statement's position
/// in the list passed to `run_statements`.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum RunEvent {
    StatementStarted {
        index: u32,
    },
    /// Before any `rows`; only for statements that return rows.
    Columns {
        index: u32,
        columns: Vec<ColumnMeta>,
    },
    Rows {
        index: u32,
        rows: Vec<Vec<Value>>,
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
        /// Rows returned (as far as they were read) or changed.
        rows: u64,
        elapsed_ms: f64,
        /// The row limit was reached and the rest was not fetched.
        truncated: bool,
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

fn ms(elapsed: Duration) -> f64 {
    elapsed.as_secs_f64() * 1000.0
}

fn not_found(session_id: Uuid) -> DbError {
    DbError::new(
        ErrorKind::NotFound,
        format!("session {session_id} not found"),
    )
}

fn cancelled_error() -> DbError {
    DbError::new(ErrorKind::Cancelled, "Cancelled")
}

/// Runs `statements` in order on a session and stops at the first error.
/// Statements that return rows send at most `row_limit` of them (0 = all);
/// the rest is cancelled on the server.
///
/// Fails with `NotFound` for an unknown session and `Busy` while another run
/// holds the session. Statement errors are `statementFailed` events, not
/// errors of the command.
#[tauri::command]
#[specta::specta]
pub async fn run_statements(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    statements: Vec<String>,
    row_limit: u32,
    on_event: Channel<RunEvent>,
) -> Result<RunSummary, DbError> {
    let session = sessions
        .get(session_id)
        .await
        .ok_or_else(|| not_found(session_id))?;
    let _running = session.run_lock.try_lock().map_err(|_| {
        DbError::new(
            ErrorKind::Busy,
            "A query is already running in this session",
        )
    })?;
    session.cancelled.store(false, Ordering::SeqCst);

    let emit = |event: RunEvent| {
        // The window may be gone; the run still finishes so the session is left clean.
        if let Err(err) = on_event.send(event) {
            tracing::debug!(error = %err, "run event not delivered");
        }
    };
    let limit = match row_limit {
        0 => u64::MAX,
        n => u64::from(n),
    };
    Ok(run(&session, &statements, limit, &emit).await)
}

async fn run(
    session: &Session,
    statements: &[String],
    limit: u64,
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
        let result = run_one(session, index, sql, limit, emit).await;
        drain_notices(session, index, emit);
        let elapsed_ms = ms(statement_started.elapsed());
        match result {
            Ok((tag, rows, truncated)) => {
                executed += 1;
                emit(RunEvent::StatementFinished {
                    index,
                    tag,
                    rows,
                    elapsed_ms,
                    truncated,
                });
            }
            Err(error) => {
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

/// One statement: (tag, row count, truncated).
async fn run_one(
    session: &Session,
    index: u32,
    sql: &str,
    limit: u64,
    emit: &impl Fn(RunEvent),
) -> Result<(String, u64, bool), DbError> {
    let dialect = session.conn.dialect();
    match session.conn.execute(sql).await? {
        ExecOutcome::Affected { rows, command } => {
            Ok((dialect.command_tag(&command, rows), rows, false))
        }
        ExecOutcome::Rows {
            columns,
            stream,
            command,
        } => {
            emit(RunEvent::Columns { index, columns });
            let (rows, truncated) = stream_rows(session, index, stream, limit, emit).await?;
            Ok((dialect.command_tag(&command, rows), rows, truncated))
        }
    }
}

/// Sends rows until the stream ends or `limit` is reached: (rows sent, truncated).
async fn stream_rows(
    session: &Session,
    index: u32,
    mut stream: Box<dyn RowStream>,
    limit: u64,
    emit: &impl Fn(RunEvent),
) -> Result<(u64, bool), DbError> {
    let mut sent = 0u64;
    loop {
        if session.cancelled.load(Ordering::SeqCst) {
            // Rows already buffered would otherwise keep coming after Stop.
            let _ = stream.close().await;
            return Err(cancelled_error());
        }
        let want = (limit - sent).min(BATCH_SIZE as u64) as usize;
        let batch = match stream.next_batch(want).await {
            Ok(Some(batch)) => batch,
            Ok(None) => break,
            Err(err) => {
                let _ = stream.close().await;
                return Err(err);
            }
        };
        sent += batch.rows.len() as u64;
        emit(RunEvent::Rows {
            index,
            rows: batch.rows,
        });
        drain_notices(session, index, emit);
        if !batch.has_more {
            break;
        }
        if sent >= limit {
            // Cancels the query and reads up to its end, so the session is free.
            stream.close().await?;
            return Ok((sent, true));
        }
    }
    stream.close().await?;
    Ok((sent, false))
}

fn drain_notices(session: &Session, index: u32, emit: &impl Fn(RunEvent)) {
    for notice in session.conn.take_notices() {
        emit(RunEvent::Notice { index, notice });
    }
}

/// Stops the session's run, if there is one. The running statement fails
/// with `Cancelled` and the rest are not started.
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
    // that is just starting fail with Busy. With nothing running, the server
    // ignores the cancel and the next run clears the flag.
    session.cancelled.store(true, Ordering::SeqCst);
    // TODO: a cancel that reaches the server just after a statement finished can
    // hit the next one; harmless here since the run stops before starting it.
    session.conn.cancel_handle().cancel().await
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
        let summary = run_statements(app.state(), id, statements, 1000, channel)
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
        let summary = run_statements(app.state(), id, statements, 1000, channel)
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
        assert_eq!(finished[0]["truncated"], true);
        assert_eq!(finished[0]["tag"], "SELECT 1000");
        assert_eq!(finished[1]["truncated"], false);
        assert_eq!(finished[1]["tag"], "SELECT 1");
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs SWIFTQL_PG_URL"]
    async fn stops_at_first_error() {
        let app = app();
        let id = open_session(&app).await;
        let (channel, events) = recorder();
        let statements = sql(&["SELECT 1", "SELEC 1", "SELECT 2"]);
        let summary = run_statements(app.state(), id, statements, 1000, channel)
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
            run_statements(handle.state(), id, statements, 1000, channel).await
        });
        // Wait until the first run holds the lock and has sent its statement.
        while started.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let (channel, _) = recorder();
        let err = run_statements(app.state(), id, sql(&["SELECT 1"]), 1000, channel)
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
        let err = run_statements(app.state(), Uuid::new_v4(), sql(&["SELECT 1"]), 10, channel)
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
