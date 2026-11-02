//! Persistent app state: settings (`settings.json`), saved connections
//! (`app.db`) and their secrets (OS keyring). No Tauri dependency; the host
//! app passes in the directory to use.

pub mod connections;
pub mod db;
pub mod error;
pub mod paths;
pub mod secrets;
pub mod settings;

use std::{path::PathBuf, sync::Arc};

use serde_json::Value;

pub use connections::{
    ConnectionInput, ConnectionRepo, ConnectionStatus, SavedConnection, StoredAuth, StoredSshAuth,
    StoredTunnel,
};
pub use db::Db;
pub use error::{ConfigError, ConfigResult};
pub use paths::ConfigPaths;
pub use secrets::{
    KeyringBackend, KeyringStatus, MemoryBackend, SecretBackend, SecretKind, SecretRef, SecretStore,
};
pub use settings::{Settings, SettingsStore};
pub use uuid::Uuid;

/// Owns every store. `Send + Sync`, share it as `Arc<ConfigManager>`.
pub struct ConfigManager {
    paths: ConfigPaths,
    settings: SettingsStore,
    db: Db,
    secrets: SecretStore,
}

impl ConfigManager {
    /// Opens (and creates) the stores under `root`, with secrets in the OS keyring.
    pub fn open(root: impl Into<PathBuf>) -> ConfigResult<Self> {
        Self::open_with(root, Arc::new(KeyringBackend))
    }

    pub fn open_with(
        root: impl Into<PathBuf>,
        backend: Arc<dyn SecretBackend>,
    ) -> ConfigResult<Self> {
        let paths = ConfigPaths::new(root);
        paths.ensure()?;
        let settings = SettingsStore::load(&paths.settings_file)?;
        let db = Db::open(&paths.db_file)?;
        tracing::debug!(root = %paths.root.display(), "opened config store");
        Ok(Self {
            paths,
            settings,
            db,
            secrets: SecretStore::new(backend),
        })
    }

    pub fn paths(&self) -> &ConfigPaths {
        &self.paths
    }

    pub fn settings(&self) -> &SettingsStore {
        &self.settings
    }

    pub fn connections(&self) -> ConnectionRepo<'_> {
        ConnectionRepo::new(&self.db, &self.secrets)
    }

    pub fn secrets(&self) -> &SecretStore {
        &self.secrets
    }

    /// Schemaless app state (last selected connection, layout, ...).
    pub fn app_state(&self, key: &str) -> ConfigResult<Option<Value>> {
        self.db.kv_get(key)
    }

    /// `null` removes the key.
    pub fn set_app_state(&self, key: &str, value: &Value) -> ConfigResult<()> {
        self.db.kv_set(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn manager_is_send_sync() {
        assert_send_sync::<ConfigManager>();
    }

    #[test]
    fn open_creates_root_and_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("nested").join("swiftql");
        let manager = ConfigManager::open_with(&root, Arc::new(MemoryBackend::new())).unwrap();
        assert!(root.is_dir());
        assert!(manager.paths().db_file.exists());
        manager
            .settings()
            .update(serde_json::json!({ "editor": { "fontSize": 15 } }))
            .unwrap();
        assert!(manager.paths().settings_file.exists());
        manager.set_app_state("lastConnection", &"abc".into()).unwrap();
        assert_eq!(
            manager.app_state("lastConnection").unwrap(),
            Some(Value::from("abc"))
        );
    }
}
