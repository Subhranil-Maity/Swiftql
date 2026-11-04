//! Saving exported results and diagrams. The frontend picks the path with the
//! dialog plugin's save dialog, then sends the file's bytes here.

/// Writes `bytes` to `path`, replacing the file if it exists (the save dialog
/// has already asked about overwriting).
// TODO: bytes arrive as a JSON number array; switch to a raw IPC body for large exports
#[tauri::command]
#[specta::specta]
pub async fn write_export_file(path: String, bytes: Vec<u8>) -> Result<(), String> {
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|err| format!("Couldn't write {path}: {err}"))
}
