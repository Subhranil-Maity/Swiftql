use std::path::PathBuf;

use crate::error::ConfigResult;

pub const SETTINGS_FILE: &str = "settings.json";
pub const DB_FILE: &str = "app.db";

/// Where the app keeps its files. The root is chosen by the host app
/// (Tauri passes `app_config_dir()`).
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub root: PathBuf,
    pub settings_file: PathBuf,
    pub db_file: PathBuf,
}

impl ConfigPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            settings_file: root.join(SETTINGS_FILE),
            db_file: root.join(DB_FILE),
            root,
        }
    }

    /// Creates the root directory if it does not exist yet.
    pub fn ensure(&self) -> ConfigResult<()> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }
}
