//! Results held in the backend. A statement's rows are read into a buffer
//! (up to the `query.maxRowsHeld` / `query.maxResultMb` caps) and the grid
//! pages through them with `fetch_rows`, so results of any size can be
//! scrolled, sorted and exported without sending them all to the webview.

mod format;
mod sort;
mod text;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, RwLock},
};

use driver_core::{
    error::{DbError, ErrorKind},
    types::ColumnMeta,
    value::Value,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use uuid::Uuid;

pub use format::ExportFormat;
pub use sort::{SortDirection, sort_order};

use crate::{
    export::{SaveFilter, SaveOptions, pick_save_path},
    session::SessionManager,
};

/// Results kept per session; the oldest finished ones are dropped past this.
/// The frontend also releases results when a tab re-runs or closes.
const MAX_RESULTS_PER_SESSION: usize = 64;

/// Most rows `fetch_rows` returns at once.
const MAX_PAGE: u32 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ResultState {
    /// Rows are still being read.
    Loading,
    /// Every row was read.
    Complete,
    /// Reading stopped at the row or size cap; the rest wasn't read.
    Truncated,
    /// Stopped by the user; the rows read so far are kept.
    Cancelled,
    /// The statement failed while its rows were read.
    Failed,
}

pub struct ResultBuffer {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Vec<Value>>,
    /// Rough size of `rows` in memory.
    pub est_bytes: u64,
    pub state: ResultState,
    /// Display position → row index while sorted.
    order: Option<Vec<u32>>,
}

impl ResultBuffer {
    pub fn new(columns: Vec<ColumnMeta>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            est_bytes: 0,
            state: ResultState::Loading,
            order: None,
        }
    }

    pub fn push_rows(&mut self, rows: Vec<Vec<Value>>) {
        self.est_bytes += rows.iter().map(|r| row_size(r)).sum::<u64>();
        self.rows.extend(rows);
    }

    /// The row shown at display position `pos`.
    fn row_at(&self, pos: usize) -> Option<&Vec<Value>> {
        match &self.order {
            Some(order) => order.get(pos).and_then(|&i| self.rows.get(i as usize)),
            None => self.rows.get(pos),
        }
    }
}

pub type SharedBuffer = Arc<RwLock<ResultBuffer>>;

/// One session's results, oldest first.
#[derive(Default)]
pub struct ResultStore {
    entries: Mutex<VecDeque<(Uuid, SharedBuffer)>>,
}

impl ResultStore {
    pub fn insert(&self, columns: Vec<ColumnMeta>) -> (Uuid, SharedBuffer) {
        let id = Uuid::new_v4();
        let buffer = Arc::new(RwLock::new(ResultBuffer::new(columns)));
        let mut entries = lock(&self.entries);
        entries.push_back((id, Arc::clone(&buffer)));
        while entries.len() > MAX_RESULTS_PER_SESSION {
            // Never drop one that is still loading (only the running one is).
            let Some(pos) = entries
                .iter()
                .position(|(_, b)| read(b).state != ResultState::Loading)
            else {
                break;
            };
            entries.remove(pos);
        }
        (id, buffer)
    }

    pub fn get(&self, id: Uuid) -> Option<SharedBuffer> {
        lock(&self.entries)
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, b)| Arc::clone(b))
    }

    pub fn release(&self, ids: &[Uuid]) {
        lock(&self.entries).retain(|(id, _)| !ids.contains(id));
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn read(b: &SharedBuffer) -> std::sync::RwLockReadGuard<'_, ResultBuffer> {
    b.read().unwrap_or_else(|e| e.into_inner())
}

pub fn write(b: &SharedBuffer) -> std::sync::RwLockWriteGuard<'_, ResultBuffer> {
    b.write().unwrap_or_else(|e| e.into_inner())
}

/// Approximate heap size of a row, for the `maxResultMb` cap.
pub fn row_size(row: &[Value]) -> u64 {
    24 + row.iter().map(value_size).sum::<u64>()
}

fn value_size(v: &Value) -> u64 {
    const BASE: u64 = std::mem::size_of::<Value>() as u64;
    BASE + match v {
        Value::Decimal(s) | Value::BigInt(s) | Value::Text(s) | Value::Money(s) => s.len() as u64,
        Value::Bytes(b) => b.len() as u64,
        Value::Json(j) => j.to_string().len() as u64 * 2,
        Value::Range { lower, upper, .. } => {
            lower.as_deref().map_or(0, value_size) + upper.as_deref().map_or(0, value_size)
        }
        Value::Array(items) => items.iter().map(value_size).sum(),
        Value::Unknown { type_name, text } => (type_name.len() + text.len()) as u64,
        _ => 0,
    }
}

// ---------- commands ----------

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RowPage {
    /// Rows from `offset`, in display (sorted) order.
    pub rows: Vec<Vec<Value>>,
    /// Rows held so far.
    pub total: u32,
    pub state: ResultState,
}

#[derive(Debug, Clone, Copy, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResultSort {
    pub column: u32,
    pub direction: SortDirection,
}

/// Part of a result in display order: row ranges (`[start, end)`; all rows
/// when absent) and source column indices.
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResultSelection {
    pub rows: Option<Vec<(u32, u32)>>,
    pub columns: Vec<u32>,
}

fn result_not_found() -> DbError {
    DbError::new(
        ErrorKind::NotFound,
        "This result is no longer held; run the query again",
    )
}

async fn buffer_of(
    sessions: &SessionManager,
    session_id: Uuid,
    result_id: Uuid,
) -> Result<SharedBuffer, DbError> {
    let session = sessions.get(session_id).await.ok_or_else(|| {
        DbError::new(
            ErrorKind::NotFound,
            format!("session {session_id} not found"),
        )
    })?;
    session.results.get(result_id).ok_or_else(result_not_found)
}

/// Rows `offset..offset + limit` of a result in display order. Works while
/// the result is still loading; `total` says how many rows are held so far.
#[tauri::command]
#[specta::specta]
pub async fn fetch_rows(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    result_id: Uuid,
    offset: u32,
    limit: u32,
) -> Result<RowPage, DbError> {
    let buffer = buffer_of(&sessions, session_id, result_id).await?;
    let b = read(&buffer);
    let start = offset as usize;
    let end = start.saturating_add(limit.min(MAX_PAGE) as usize);
    let rows = (start..end)
        .map_while(|pos| b.row_at(pos).cloned())
        .collect();
    Ok(RowPage {
        rows,
        total: b.rows.len() as u32,
        state: b.state,
    })
}

/// Sorts the whole held result (type-aware, NULLs last, stable), or restores
/// the original order with `None`. Fails with `Busy` while it is loading.
#[tauri::command]
#[specta::specta]
pub async fn sort_result(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    result_id: Uuid,
    sort: Option<ResultSort>,
) -> Result<(), DbError> {
    let buffer = buffer_of(&sessions, session_id, result_id).await?;
    if read(&buffer).state == ResultState::Loading {
        return Err(DbError::new(
            ErrorKind::Busy,
            "Rows are still loading; sort when they're done",
        ));
    }
    // Sorting a large result takes a while; keep it off the async workers.
    tokio::task::spawn_blocking(move || {
        let order = sort.map(|s| sort_order(&read(&buffer).rows, s.column as usize, s.direction));
        write(&buffer).order = order;
    })
    .await
    .map_err(|err| DbError::new(ErrorKind::Internal, err.to_string()))
}

/// Forgets results the UI no longer shows (a tab re-ran or closed).
#[tauri::command]
#[specta::specta]
pub async fn release_results(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    result_ids: Vec<Uuid>,
) -> Result<(), DbError> {
    if let Some(session) = sessions.get(session_id).await {
        session.results.release(&result_ids);
    }
    Ok(())
}

/// Part of a result as text in `format`, for Copy as.
#[tauri::command]
#[specta::specta]
pub async fn copy_result(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    result_id: Uuid,
    format: ExportFormat,
    selection: ResultSelection,
) -> Result<String, DbError> {
    let buffer = buffer_of(&sessions, session_id, result_id).await?;
    tokio::task::spawn_blocking(move || {
        let mut out = Vec::new();
        format::write(&read(&buffer), &selection, format, &mut out)
            .map_err(|err| DbError::new(ErrorKind::Internal, err.to_string()))?;
        String::from_utf8(out).map_err(|err| DbError::new(ErrorKind::Internal, err.to_string()))
    })
    .await
    .map_err(|err| DbError::new(ErrorKind::Internal, err.to_string()))?
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportOutcome {
    pub path: String,
    pub rows: u32,
}

/// Asks where to save, then writes part of a result there in `format`.
/// None when the dialog is cancelled.
#[tauri::command]
#[specta::specta]
pub async fn export_result(
    app: tauri::AppHandle,
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
    result_id: Uuid,
    format: ExportFormat,
    selection: ResultSelection,
    default_name: String,
) -> Result<Option<ExportOutcome>, DbError> {
    let buffer = buffer_of(&sessions, session_id, result_id).await?;
    let options = SaveOptions {
        default_name,
        filters: vec![SaveFilter {
            name: format.label().into(),
            extensions: vec![format.extension().into()],
        }],
    };
    let internal = |err: String| DbError::new(ErrorKind::Internal, err);
    let Some(path) = pick_save_path(&app, &options).await.map_err(internal)? else {
        return Ok(None);
    };
    let shown = path.display().to_string();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&path)
            .map_err(|err| format!("Couldn't write {}: {err}", path.display()))?;
        let mut out = std::io::BufWriter::new(file);
        let rows = format::write(&read(&buffer), &selection, format, &mut out)
            .and_then(|rows| std::io::Write::flush(&mut out).map(|_| rows))
            .map_err(|err| format!("Couldn't write {}: {err}", path.display()))?;
        Ok::<_, String>(rows)
    })
    .await
    .map_err(|err| internal(err.to_string()))?
    .map_err(internal)
    .map(|rows| Some(ExportOutcome { path: shown, rows }))
}
