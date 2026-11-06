//! Saving exported results and diagrams. The backend opens the native "Save
//! as" dialog itself and writes only to the path the user picked there, so the
//! webview can never choose where a file is written.

use serde::Deserialize;
use tauri::ipc::{InvokeBody, Request};
use tauri_plugin_dialog::DialogExt;

/// Name of the header carrying [`SaveOptions`] as URI-encoded JSON.
const OPTIONS_HEADER: &str = "x-swiftql-save";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOptions {
    pub default_name: String,
    #[serde(default)]
    pub filters: Vec<SaveFilter>,
}

/// Asks where to save, then writes `bytes` there (the dialog has already
/// asked about overwriting). Returns the path, or None when cancelled.
pub async fn save_with_dialog(
    app: &tauri::AppHandle,
    options: &SaveOptions,
    bytes: &[u8],
) -> Result<Option<String>, String> {
    let Some(path) = pick_save_path(app, options).await? else {
        return Ok(None);
    };
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|err| format!("Couldn't write {}: {err}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

/// Shows the save dialog. None when the user cancels.
pub async fn pick_save_path(
    app: &tauri::AppHandle,
    options: &SaveOptions,
) -> Result<Option<std::path::PathBuf>, String> {
    let mut dialog = app.dialog().file().set_file_name(&options.default_name);
    for filter in &options.filters {
        let exts: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
        dialog = dialog.add_filter(&filter.name, &exts);
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    dialog.save_file(move |picked| {
        let _ = tx.send(picked);
    });
    let picked = rx
        .await
        .map_err(|_| "The save dialog closed unexpectedly".to_string())?;
    match picked {
        None => Ok(None),
        Some(file) => file
            .into_path()
            .map(Some)
            .map_err(|err| format!("Unsupported save location: {err}")),
    }
}

fn options_from(request: &Request<'_>) -> Result<SaveOptions, String> {
    let raw = request
        .headers()
        .get(OPTIONS_HEADER)
        .ok_or("Missing save options")?
        .to_str()
        .map_err(|_| "Save options aren't ASCII")?;
    let json = percent_decode(raw).ok_or("Save options aren't valid URI encoding")?;
    serde_json::from_str(&json).map_err(|err| format!("Invalid save options: {err}"))
}

/// Decodes `encodeURIComponent` output.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// `save_export`: the body is the file's raw bytes (not a JSON number array),
/// the options travel in the `x-swiftql-save` header. Not a specta command
/// because it reads a raw body; `src/lib/ipc.ts` wraps it by hand.
#[tauri::command]
pub async fn save_export(
    app: tauri::AppHandle,
    request: Request<'_>,
) -> Result<Option<String>, String> {
    let options = options_from(&request)?;
    let bytes = match request.body() {
        InvokeBody::Raw(bytes) => bytes.as_slice(),
        InvokeBody::Json(_) => return Err("Expected a raw byte body".into()),
    };
    save_with_dialog(&app, &options, bytes).await
}

#[cfg(test)]
mod tests {
    use super::percent_decode;

    #[test]
    fn decodes_uri_components() {
        assert_eq!(
            percent_decode("a%20b%7B%22x%22%7D").as_deref(),
            Some("a b{\"x\"}")
        );
        assert_eq!(percent_decode("%E2%82%AC").as_deref(), Some("€"));
        assert_eq!(percent_decode("%2"), None);
    }
}
