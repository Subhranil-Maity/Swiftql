use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::RwLock,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ConfigError, ConfigResult};

/// Bump when the file layout changes and add a migration in `SettingsStore::load`.
pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub general: GeneralSettings,
    pub editor: EditorSettings,
    pub query: QuerySettings,
    pub connection: ConnectionSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            general: GeneralSettings::default(),
            editor: EditorSettings::default(),
            query: QuerySettings::default(),
            connection: ConnectionSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct GeneralSettings {
    pub theme: Theme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: u32,
    pub tab_size: u32,
    pub word_wrap: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: "ui-monospace, monospace".into(),
            font_size: 14,
            tab_size: 2,
            word_wrap: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct QuerySettings {
    pub row_limit: u32,
    pub query_timeout_secs: u32, // 0 = no timeout
    pub confirm_destructive: bool,
}

impl Default for QuerySettings {
    fn default() -> Self {
        Self {
            row_limit: 1000,
            query_timeout_secs: 0,
            confirm_destructive: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct ConnectionSettings {
    pub connect_timeout_secs: u32,
    pub keepalive_secs: u32, // 0 = off
    /// How many recently used connections to health-check when the app
    /// opens. 0 = off.
    pub check_on_startup: u32,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 10,
            keepalive_secs: 60,
            check_on_startup: 10,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> ConfigResult<()> {
        fn check(ok: bool, msg: &str) -> ConfigResult<()> {
            if ok { Ok(()) } else { Err(ConfigError::Invalid(msg.into())) }
        }
        check(
            (6..=72).contains(&self.editor.font_size),
            "editor.fontSize must be between 6 and 72",
        )?;
        check(
            (1..=16).contains(&self.editor.tab_size),
            "editor.tabSize must be between 1 and 16",
        )?;
        check(
            !self.editor.font_family.trim().is_empty(),
            "editor.fontFamily must not be empty",
        )?;
        check(self.query.row_limit >= 1, "query.rowLimit must be at least 1")?;
        check(
            (1..=600).contains(&self.connection.connect_timeout_secs),
            "connection.connectTimeoutSecs must be between 1 and 600",
        )?;
        check(
            self.connection.check_on_startup <= 50,
            "connection.checkOnStartup must be between 0 and 50",
        )?;
        Ok(())
    }
}

/// `settings.json`, kept in memory and written back atomically on every change.
pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<Settings>,
}

impl SettingsStore {
    /// A missing file gives defaults. A corrupt or invalid file is moved to
    /// `settings.json.bak` and defaults are used.
    pub fn load(path: impl Into<PathBuf>) -> ConfigResult<Self> {
        let path = path.into();
        let settings = match fs::read_to_string(&path) {
            Ok(text) => match parse(&text) {
                Ok(settings) => settings,
                Err(err) => {
                    let backup = with_suffix(&path, ".bak");
                    tracing::warn!(
                        error = %err,
                        backup = %backup.display(),
                        "settings file is unreadable, falling back to defaults"
                    );
                    fs::rename(&path, &backup)?;
                    Settings::default()
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(err) => return Err(err.into()),
        };
        Ok(Self {
            path,
            current: RwLock::new(settings),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self) -> Settings {
        self.current.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Deep-merges `patch` into the current settings, validates, saves and
    /// returns the result. A `null` in the patch resets that field to its default.
    pub fn update(&self, patch: Value) -> ConfigResult<Settings> {
        if !patch.is_object() {
            return Err(ConfigError::Invalid("settings patch must be an object".into()));
        }
        let mut current = self.current.write().unwrap_or_else(|e| e.into_inner());
        let mut merged = serde_json::to_value(&*current)?;
        merge(&mut merged, patch);
        let mut next: Settings = serde_json::from_value(merged)?;
        next.version = SETTINGS_VERSION;
        next.validate()?;
        save(&self.path, &next)?;
        *current = next.clone();
        Ok(next)
    }

    pub fn reset(&self) -> ConfigResult<Settings> {
        let mut current = self.current.write().unwrap_or_else(|e| e.into_inner());
        let next = Settings::default();
        save(&self.path, &next)?;
        *current = next.clone();
        Ok(next)
    }
}

fn parse(text: &str) -> ConfigResult<Settings> {
    let settings: Settings = serde_json::from_str(text)?;
    // TODO: migrate older `version`s here once SETTINGS_VERSION goes past 1
    settings.validate()?;
    Ok(settings)
}

fn merge(target: &mut Value, patch: Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                if value.is_null() {
                    target.remove(&key);
                } else {
                    merge(target.entry(key).or_insert(Value::Null), value);
                }
            }
        }
        (target, patch) => *target = patch,
    }
}

/// Writes to `<file>.tmp` and renames it over the real file.
fn save(path: &Path, settings: &Settings) -> ConfigResult<()> {
    let tmp = with_suffix(path, ".tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(&serde_json::to_vec_pretty(settings)?)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store(dir: &tempfile::TempDir) -> SettingsStore {
        SettingsStore::load(dir.path().join("settings.json")).unwrap()
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(store(&dir).get(), Settings::default());
        assert!(!dir.path().join("settings.json").exists());
    }

    #[test]
    fn defaults() {
        let s = Settings::default();
        assert_eq!(s.version, SETTINGS_VERSION);
        assert_eq!(s.connection.connect_timeout_secs, 10);
        assert_eq!(s.connection.keepalive_secs, 60);
        assert_eq!(s.connection.check_on_startup, 10);
        assert!(s.validate().is_ok());
    }

    #[test]
    fn older_file_without_check_on_startup_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{ "version": 1, "connection": { "connectTimeoutSecs": 5 } }"#).unwrap();
        let s = store(&dir).get();
        assert_eq!(s.connection.connect_timeout_secs, 5);
        assert_eq!(s.connection.check_on_startup, 10);
        assert!(path.exists(), "a valid file must not be moved to .bak");
    }

    #[test]
    fn check_on_startup_is_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(&dir);
        let set = |n: i64| s.update(json!({ "connection": { "checkOnStartup": n } }));
        assert_eq!(set(0).unwrap().connection.check_on_startup, 0);
        assert_eq!(set(50).unwrap().connection.check_on_startup, 50);
        assert!(matches!(set(51).unwrap_err(), ConfigError::Invalid(_)));
        assert!(matches!(set(-1).unwrap_err(), ConfigError::Json(_)));
        assert_eq!(s.get().connection.check_on_startup, 50);
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let saved = store(&dir)
            .update(json!({ "general": { "theme": "dark" }, "query": { "rowLimit": 50 } }))
            .unwrap();
        let reloaded = store(&dir).get();
        assert_eq!(saved, reloaded);
        assert_eq!(reloaded.general.theme, Theme::Dark);
        assert_eq!(reloaded.query.row_limit, 50);
    }

    #[test]
    fn patch_is_deep_merged() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(&dir);
        s.update(json!({ "editor": { "wordWrap": true } })).unwrap();
        let next = s.update(json!({ "editor": { "fontSize": 15 } })).unwrap();
        assert!(next.editor.word_wrap);
        assert_eq!(next.editor.font_size, 15);
        assert_eq!(next.editor.tab_size, EditorSettings::default().tab_size);

        let reset = s.update(json!({ "editor": { "fontSize": null } })).unwrap();
        assert_eq!(reset.editor.font_size, EditorSettings::default().font_size);
        assert!(reset.editor.word_wrap);
    }

    #[test]
    fn invalid_patch_is_rejected_and_not_saved() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(&dir);
        let err = s.update(json!({ "editor": { "fontSize": 1000 } })).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
        let err = s.update(json!({ "editor": { "fontSize": "big" } })).unwrap_err();
        assert!(matches!(err, ConfigError::Json(_)));
        assert_eq!(s.get(), Settings::default());
        assert!(!s.path().exists());
    }

    #[test]
    fn unknown_fields_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{ "version": 1, "bogus": 1, "editor": { "tabSize": 4 } }"#).unwrap();
        let s = store(&dir);
        assert_eq!(s.get().editor.tab_size, 4);
        s.update(json!({})).unwrap();
        let on_disk: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(on_disk.get("bogus").is_none());
    }

    #[test]
    fn corrupt_file_is_backed_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{ not json").unwrap();
        let s = store(&dir);
        assert_eq!(s.get(), Settings::default());
        assert!(!path.exists());
        let backup = dir.path().join("settings.json.bak");
        assert_eq!(fs::read_to_string(backup).unwrap(), "{ not json");
    }

    #[test]
    fn save_is_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(&dir);
        s.update(json!({ "editor": { "fontSize": 16 } })).unwrap();
        s.update(json!({ "editor": { "fontSize": 18 } })).unwrap();
        assert!(!dir.path().join("settings.json.tmp").exists());
        let on_disk: Settings =
            serde_json::from_str(&fs::read_to_string(s.path()).unwrap()).unwrap();
        assert_eq!(on_disk.editor.font_size, 18);
    }

    #[test]
    fn reset_restores_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(&dir);
        s.update(json!({ "general": { "theme": "light" } })).unwrap();
        assert_eq!(s.reset().unwrap(), Settings::default());
        assert_eq!(store(&dir).get(), Settings::default());
    }
}
