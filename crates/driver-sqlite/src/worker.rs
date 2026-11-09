//! A rusqlite connection lives on its own OS thread; async callers send it
//! jobs and await the reply. A statement that returns rows stays open on the
//! thread between `Fetch` jobs, so rows are read as the app asks for them.

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use driver_core::{
    connection::RowBatch,
    dbresults::DbResult,
    error::{DbError, ErrorKind},
    types::{ColumnMeta, LogicalType},
    value::Value,
};
use rusqlite::{Connection, OpenFlags};
use tokio::sync::oneshot;

use crate::{
    command::{command_of, counts_changes},
    error::map_err,
    types::{column_meta, decode},
};

/// How many SQLite VM steps between deadline checks.
const PROGRESS_STEPS: i32 = 1000;

/// Waits this long for another process's lock before failing with SQLITE_BUSY.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// State shared between the worker thread and the async side.
#[derive(Default)]
pub struct Shared {
    /// `!is_autocommit()` after the last job.
    pub in_transaction: AtomicBool,
    /// Query timeout in seconds; 0 = none.
    pub timeout_secs: AtomicU32,
    /// When the running statement times out.
    deadline: Mutex<Option<Instant>>,
    /// Set by the progress handler when it interrupted for the timeout.
    timed_out: AtomicBool,
}

impl Shared {
    fn start_statement(&self) {
        self.timed_out.store(false, Ordering::SeqCst);
        let secs = self.timeout_secs.load(Ordering::SeqCst);
        *self.deadline.lock().unwrap_or_else(|e| e.into_inner()) =
            (secs > 0).then(|| Instant::now() + Duration::from_secs(secs.into()));
    }

    fn end_statement(&self) {
        *self.deadline.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    fn timed_out(&self) -> bool {
        self.timed_out.load(Ordering::SeqCst)
    }

    /// True stops the running statement with SQLITE_INTERRUPT.
    fn past_deadline(&self) -> bool {
        let deadline = *self.deadline.lock().unwrap_or_else(|e| e.into_inner());
        let past = deadline.is_some_and(|d| Instant::now() >= d);
        if past {
            self.timed_out.store(true, Ordering::SeqCst);
        }
        past
    }
}

pub type Call = Box<dyn FnOnce(&Connection) + Send>;

pub enum Job {
    /// Run a closure on the connection (introspection, ping, ...).
    Call(Call),
    Execute {
        sql: String,
        stream_id: u64,
        reply: oneshot::Sender<DbResult<Started>>,
    },
    Fetch {
        stream_id: u64,
        max: usize,
        reply: oneshot::Sender<DbResult<Option<RowBatch>>>,
    },
    /// Finish a row statement early. `reply` is None when sent from Drop.
    CloseStream {
        stream_id: u64,
        reply: Option<oneshot::Sender<()>>,
    },
}

pub enum Started {
    Rows {
        columns: Vec<ColumnMeta>,
        command: String,
    },
    Affected {
        rows: u64,
        command: String,
    },
}

pub struct Opened {
    pub jobs: mpsc::Sender<Job>,
    pub interrupt: rusqlite::InterruptHandle,
    pub shared: Arc<Shared>,
}

/// Opens the database on a new worker thread. A missing file is an error:
/// new databases are created on purpose (the app's "New database" button).
pub async fn open(path: PathBuf, read_only: bool) -> DbResult<Opened> {
    if !path.is_file() {
        return Err(DbError::new(
            ErrorKind::NotFound,
            format!("Database file not found: {}", path.display()),
        )
        .with_hint("Check the path, or create the file with New database."));
    }
    let (opened_tx, opened_rx) = oneshot::channel();
    thread::Builder::new()
        .name("sqlite-worker".into())
        .spawn(move || {
            let shared = Arc::new(Shared::default());
            let conn = match open_connection(&path, read_only, &shared) {
                Ok(conn) => conn,
                Err(err) => {
                    let _ = opened_tx.send(Err(err));
                    return;
                }
            };
            let (jobs_tx, jobs_rx) = mpsc::channel();
            let opened = Opened {
                jobs: jobs_tx,
                interrupt: conn.get_interrupt_handle(),
                shared: Arc::clone(&shared),
            };
            if opened_tx.send(Ok(opened)).is_ok() {
                Worker {
                    conn,
                    shared,
                    jobs: jobs_rx,
                    pending: VecDeque::new(),
                }
                .run();
            }
        })
        .map_err(|err| {
            DbError::new(ErrorKind::Internal, format!("couldn't start SQLite: {err}"))
        })?;
    opened_rx.await.map_err(|_| gone())?
}

fn open_connection(path: &PathBuf, read_only: bool, shared: &Arc<Shared>) -> DbResult<Connection> {
    let access = if read_only {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    };
    // No SQLITE_OPEN_CREATE: the file must exist (checked above, but it can
    // disappear in between).
    let flags = access | OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_URI;
    let mut conn = Connection::open_with_flags(path, flags).map_err(|e| map_err(e, false))?;
    conn.busy_timeout(BUSY_TIMEOUT)
        .map_err(|e| map_err(e, false))?;
    // Off by default in SQLite for compatibility; a client should enforce them.
    conn.execute_batch("PRAGMA foreign_keys = ON")
        .map_err(|e| map_err(e, false))?;
    // A file that isn't a database only fails on first read.
    conn.query_row("SELECT count(*) FROM sqlite_schema", [], |_| Ok(()))
        .map_err(|e| map_err(e, false))?;
    let handler_state = Arc::clone(shared);
    conn.progress_handler(PROGRESS_STEPS, Some(move || handler_state.past_deadline()))
        .map_err(|e| map_err(e, false))?;
    Ok(conn)
}

/// Records whether a transaction is open. Called before every reply, so the
/// caller sees the status its own statement left behind.
pub fn track_tx(conn: &Connection, shared: &Shared) {
    shared
        .in_transaction
        .store(!conn.is_autocommit(), Ordering::SeqCst);
}

pub fn gone() -> DbError {
    DbError::new(ErrorKind::Network, "the SQLite connection is closed")
}

struct Worker {
    conn: Connection,
    shared: Arc<Shared>,
    jobs: mpsc::Receiver<Job>,
    /// Jobs that arrived while a row statement was open; run after it.
    pending: VecDeque<Job>,
}

impl Worker {
    fn run(mut self) {
        loop {
            let job = match self.pending.pop_front() {
                Some(job) => job,
                None => match self.jobs.recv() {
                    Ok(job) => job,
                    Err(_) => return, // every sender dropped: connection closed
                },
            };
            self.handle(job);
        }
    }

    fn handle(&mut self, job: Job) {
        match job {
            // The closure replies itself, so it records the status first
            // (`SqliteConnection::call`).
            Job::Call(call) => call(&self.conn),
            Job::Execute {
                sql,
                stream_id,
                reply,
            } => self.execute(&sql, stream_id, reply),
            // The stream already ended (or never started).
            Job::Fetch { reply, .. } => {
                let _ = reply.send(Ok(None));
            }
            Job::CloseStream { reply, .. } => {
                if let Some(reply) = reply {
                    let _ = reply.send(());
                }
            }
        }
    }

    fn execute(&mut self, sql: &str, stream_id: u64, reply: oneshot::Sender<DbResult<Started>>) {
        self.shared.start_statement();
        let shared = Arc::clone(&self.shared);
        let err = |e: rusqlite::Error| map_err(e, shared.timed_out());

        let mut stmt = match self.conn.prepare(sql) {
            Ok(stmt) => stmt,
            Err(e) => {
                self.shared.end_statement();
                track_tx(&self.conn, &self.shared);
                let _ = reply.send(Err(err(e)));
                return;
            }
        };

        if stmt.column_count() == 0 {
            let command = command_of(sql, false);
            let result = stmt.raw_execute().map_err(err).map(|_| {
                let rows = if counts_changes(&command) {
                    self.conn.changes()
                } else {
                    0
                };
                Started::Affected { rows, command }
            });
            drop(stmt);
            self.shared.end_statement();
            track_tx(&self.conn, &self.shared);
            let _ = reply.send(result);
            return;
        }

        let columns: Vec<ColumnMeta> = stmt
            .columns()
            .map(|c| column_meta(c.name(), c.decl_type()))
            .collect();
        let logical: Vec<LogicalType> = columns.iter().map(|c| c.logical.clone()).collect();
        let started = Started::Rows {
            columns,
            command: command_of(sql, true),
        };
        if reply.send(Ok(started)).is_err() {
            self.shared.end_statement();
            return; // the caller is gone
        }

        let mut rows = stmt.raw_query();
        // One row read ahead so `has_more` is exact.
        let mut peeked: Option<Vec<Value>> = None;
        let mut done = false;
        let mut close_reply: Option<oneshot::Sender<()>> = None;
        let read_row = |rows: &mut rusqlite::Rows<'_>| -> DbResult<Option<Vec<Value>>> {
            match rows.next() {
                Ok(Some(row)) => Ok(Some(
                    (0..logical.len())
                        .map(|i| match row.get_ref(i) {
                            Ok(v) => decode(v, &logical[i]),
                            Err(_) => Value::Null,
                        })
                        .collect(),
                )),
                Ok(None) => Ok(None),
                Err(e) => Err(map_err(e, shared.timed_out())),
            }
        };

        while !done {
            let job = match self.jobs.recv() {
                Ok(job) => job,
                Err(_) => break,
            };
            match job {
                Job::Fetch {
                    stream_id: id,
                    max,
                    reply,
                } if id == stream_id => {
                    let mut batch = Vec::with_capacity(max.max(1));
                    batch.extend(peeked.take());
                    let mut failure = None;
                    while batch.len() < max.max(1) {
                        match read_row(&mut rows) {
                            Ok(Some(row)) => batch.push(row),
                            Ok(None) => {
                                done = true;
                                break;
                            }
                            Err(e) => {
                                failure = Some(e);
                                break;
                            }
                        }
                    }
                    if failure.is_none() && !done {
                        match read_row(&mut rows) {
                            Ok(Some(row)) => peeked = Some(row),
                            Ok(None) => done = true,
                            Err(e) => failure = Some(e),
                        }
                    }
                    let result = match failure {
                        // e.g. a division by zero halfway through, or Stop
                        Some(e) => {
                            done = true;
                            Err(e)
                        }
                        None if batch.is_empty() => Ok(None),
                        None => Ok(Some(RowBatch {
                            rows: batch,
                            has_more: !done,
                        })),
                    };
                    track_tx(&self.conn, &self.shared);
                    let _ = reply.send(result);
                }
                Job::CloseStream {
                    stream_id: id,
                    reply,
                } if id == stream_id => {
                    // Answered once the statement is released, below.
                    close_reply = reply;
                    done = true;
                }
                // A stale fetch or close from an earlier stream.
                Job::Fetch { reply, .. } => {
                    let _ = reply.send(Ok(None));
                }
                Job::CloseStream { reply, .. } => {
                    if let Some(reply) = reply {
                        let _ = reply.send(());
                    }
                }
                other => self.pending.push_back(other),
            }
        }
        drop(rows);
        drop(stmt);
        self.shared.end_statement();
        track_tx(&self.conn, &self.shared);
        if let Some(reply) = close_reply {
            let _ = reply.send(());
        }
    }
}
