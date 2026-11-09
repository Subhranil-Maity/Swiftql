//! New file-based databases (SQLite). Like exports, the backend opens the
//! "Save as" dialog itself, so the webview never chooses where a file is made.

use driver_core::error::{DbError, ErrorKind};

use crate::export::{SaveFilter, SaveOptions, pick_save_path};

/// Asks where to put a new SQLite database and creates it there as an empty
/// file (an empty file is a valid empty database). Returns the path, or None
/// when the dialog was cancelled. An existing file is never overwritten.
#[tauri::command]
#[specta::specta]
pub async fn create_database_file(app: tauri::AppHandle) -> Result<Option<String>, DbError> {
    let options = SaveOptions {
        default_name: "database.db".into(),
        filters: vec![SaveFilter {
            name: "SQLite database".into(),
            extensions: ["db", "sqlite", "sqlite3", "db3"]
                .map(String::from)
                .to_vec(),
        }],
    };
    let Some(path) = pick_save_path(&app, &options)
        .await
        .map_err(|msg| DbError::new(ErrorKind::Internal, msg))?
    else {
        return Ok(None);
    };
    match tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await
    {
        Ok(_) => Ok(Some(path.display().to_string())),
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Err(DbError::new(
            ErrorKind::Unsupported,
            format!("{} already exists", path.display()),
        )
        .with_hint("To open an existing database, pick it with Browse instead.")),
        Err(err) => Err(DbError::new(
            ErrorKind::Permission,
            format!("Couldn't create {}: {err}", path.display()),
        )),
    }
}
