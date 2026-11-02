//! IPC commands over `config_store::ConfigManager`.
//!
//! Keyring and SQLite calls block, so every command runs on the blocking pool.

use std::sync::Arc;

use config_store::{
    ConfigError, ConfigManager, ConfigResult, ConnectionInput, ConnectionStatus, KeyringBackend,
    KeyringStatus, SavedConnection, SecretKind, Settings, Uuid,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
use tauri::{AppHandle, State};
use tauri_specta::Event;

pub type Config = Arc<ConfigManager>;

/// Emitted after every settings change, to every window.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SettingsChanged(pub Settings);

async fn blocking<T, F>(config: &State<'_, Config>, f: F) -> ConfigResult<T>
where
    T: Send + 'static,
    F: FnOnce(&ConfigManager) -> ConfigResult<T> + Send + 'static,
{
    let config = Arc::clone(config);
    tauri::async_runtime::spawn_blocking(move || f(&config))
        .await
        .map_err(|e| ConfigError::Io(std::io::Error::other(e)))?
}

fn emit_settings(app: &AppHandle, settings: &Settings) {
    if let Err(err) = SettingsChanged(settings.clone()).emit(app) {
        tracing::warn!(error = %err, "failed to emit {}", SettingsChanged::NAME);
    }
}

// ---------- settings ----------

#[tauri::command]
#[specta::specta]
pub async fn get_settings(config: State<'_, Config>) -> ConfigResult<Settings> {
    blocking(&config, |c| Ok(c.settings().get())).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_settings(
    app: AppHandle,
    config: State<'_, Config>,
    patch: Value,
) -> ConfigResult<Settings> {
    let settings = blocking(&config, move |c| c.settings().update(patch)).await?;
    emit_settings(&app, &settings);
    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub async fn reset_settings(app: AppHandle, config: State<'_, Config>) -> ConfigResult<Settings> {
    let settings = blocking(&config, |c| c.settings().reset()).await?;
    emit_settings(&app, &settings);
    Ok(settings)
}

// ---------- connections ----------

#[tauri::command]
#[specta::specta]
pub async fn list_connections(config: State<'_, Config>) -> ConfigResult<Vec<SavedConnection>> {
    blocking(&config, |c| c.connections().list()).await
}

#[tauri::command]
#[specta::specta]
pub async fn recent_connections(
    config: State<'_, Config>,
    limit: Option<u32>,
) -> ConfigResult<Vec<SavedConnection>> {
    blocking(&config, move |c| c.connections().recent(limit.unwrap_or(10))).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_connection(config: State<'_, Config>, id: Uuid) -> ConfigResult<SavedConnection> {
    blocking(&config, move |c| c.connections().get(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_connection(
    config: State<'_, Config>,
    input: ConnectionInput,
) -> ConfigResult<SavedConnection> {
    blocking(&config, move |c| c.connections().save(input)).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_connection(config: State<'_, Config>, id: Uuid) -> ConfigResult<()> {
    blocking(&config, move |c| c.connections().delete(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn touch_connection(config: State<'_, Config>, id: Uuid) -> ConfigResult<()> {
    blocking(&config, move |c| c.connections().touch(id)).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_connection_status(
    config: State<'_, Config>,
    id: Uuid,
    status: ConnectionStatus,
) -> ConfigResult<()> {
    blocking(&config, move |c| c.connections().set_status(id, &status)).await
}

// ---------- secrets / misc ----------

#[tauri::command]
#[specta::specta]
pub async fn clear_connection_secret(
    config: State<'_, Config>,
    id: Uuid,
    kind: SecretKind,
) -> ConfigResult<SavedConnection> {
    blocking(&config, move |c| c.connections().clear_secret(id, kind)).await
}

#[tauri::command]
#[specta::specta]
pub async fn keyring_status(config: State<'_, Config>) -> ConfigResult<KeyringStatus> {
    blocking(&config, |_| Ok(KeyringBackend::available())).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_app_state(config: State<'_, Config>, key: String) -> ConfigResult<Option<Value>> {
    blocking(&config, move |c| c.app_state(&key)).await
}

#[tauri::command]
#[specta::specta]
pub async fn set_app_state(
    config: State<'_, Config>,
    key: String,
    value: Value,
) -> ConfigResult<()> {
    blocking(&config, move |c| c.set_app_state(&key, &value)).await
}
