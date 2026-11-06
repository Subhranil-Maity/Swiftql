use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::RwLock,
};

pub use driver_core::connect::HostKeyPolicy;
use driver_core::connect::SslMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ConfigError, ConfigResult};

/// Bump when the file layout changes and add a step to `migrate`.
/// 2: `query.rowLimit` replaced by `query.pageSize` / `maxRowsHeld` / `maxResultMb`.
pub const SETTINGS_VERSION: u32 = 2;

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

/// Appearance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct GeneralSettings {
    pub theme: Theme,
    /// Base UI font size in px.
    pub ui_font_size: u32,
    pub density: Density,
    /// How NULL shows in result grids.
    pub null_display: String,
    pub boolean_display: BooleanDisplay,
    pub timestamp_display: TimestampDisplay,
    /// Thousands separators in numeric cells.
    pub number_grouping: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            ui_font_size: 14,
            density: Density::Comfortable,
            null_display: "NULL".into(),
            boolean_display: BooleanDisplay::Text,
            timestamp_display: TimestampDisplay::Iso,
            number_grouping: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum BooleanDisplay {
    /// true / false
    #[default]
    Text,
    /// t / f, like psql
    Short,
    /// ✓ / ✗
    Symbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TimestampDisplay {
    /// As the server sends it.
    #[default]
    Iso,
    /// timestamptz converted to the computer's time zone.
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum KeywordCase {
    #[default]
    Upper,
    Lower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TransactionMode {
    /// Every statement commits on its own (unless the script says BEGIN).
    #[default]
    Auto,
    /// A run starts a transaction when none is open; Commit / Rollback end it.
    Manual,
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
    /// Case of keywords inserted by autocomplete.
    pub keyword_case: KeywordCase,
    pub line_numbers: bool,
    pub autocomplete: bool,
    /// Syntax checking while typing.
    pub lint: bool,
    pub highlight_active_line: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: "ui-monospace, monospace".into(),
            font_size: 14,
            tab_size: 2,
            word_wrap: false,
            keyword_case: KeywordCase::Upper,
            line_numbers: true,
            autocomplete: true,
            lint: true,
            highlight_active_line: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct QuerySettings {
    /// Rows the grid fetches from the backend at a time.
    pub page_size: u32,
    /// Rows of one result held in memory; reading stops there. 0 = no limit.
    pub max_rows_held: u32,
    /// Estimated size of one result held in memory, in MB; reading stops there.
    pub max_result_mb: u32,
    pub query_timeout_secs: u32, // 0 = no timeout
    /// Ask before DROP, TRUNCATE, and DELETE / UPDATE without WHERE.
    pub confirm_destructive: bool,
    pub transaction_mode: TransactionMode,
}

impl Default for QuerySettings {
    fn default() -> Self {
        Self {
            page_size: 1000,
            max_rows_held: 500_000,
            max_result_mb: 512,
            query_timeout_secs: 0,
            confirm_destructive: true,
            transaction_mode: TransactionMode::Auto,
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
    /// SSL mode of new connections.
    pub default_ssl_mode: SslMode,
    pub ssh_host_key_policy: HostKeyPolicy,
    /// "Save password" starts checked in the connection form.
    pub save_passwords_by_default: bool,
    /// LISTEN notifications kept per session before the oldest are dropped.
    pub notification_buffer: u32,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            connect_timeout_secs: 10,
            keepalive_secs: 60,
            check_on_startup: 10,
            default_ssl_mode: SslMode::Prefer,
            ssh_host_key_policy: HostKeyPolicy::Ask,
            save_passwords_by_default: true,
            notification_buffer: 1000,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> ConfigResult<()> {
        fn check(ok: bool, msg: &str) -> ConfigResult<()> {
            if ok {
                Ok(())
            } else {
                Err(ConfigError::Invalid(msg.into()))
            }
        }
        check(
            (10..=24).contains(&self.general.ui_font_size),
            "general.uiFontSize must be between 10 and 24",
        )?;
        check(
            self.general.null_display.chars().count() <= 16,
            "general.nullDisplay must be at most 16 characters",
        )?;
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
        check(
            (50..=10_000).contains(&self.query.page_size),
            "query.pageSize must be between 50 and 10000",
        )?;
        check(
            (16..=16_384).contains(&self.query.max_result_mb),
            "query.maxResultMb must be between 16 and 16384",
        )?;
        check(
            self.query.query_timeout_secs <= 86_400,
            "query.queryTimeoutSecs must be at most 86400",
        )?;
        check(
            (1..=600).contains(&self.connection.connect_timeout_secs),
            "connection.connectTimeoutSecs must be between 1 and 600",
        )?;
        check(
            self.connection.keepalive_secs <= 3600,
            "connection.keepaliveSecs must be at most 3600",
        )?;
        check(
            self.connection.check_on_startup <= 50,
            "connection.checkOnStartup must be between 0 and 50",
        )?;
        check(
            (10..=100_000).contains(&self.connection.notification_buffer),
            "connection.notificationBuffer must be between 10 and 100000",
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
        self.current
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Deep-merges `patch` into the current settings, validates, saves and
    /// returns the result. A `null` in the patch resets that field to its default.
    pub fn update(&self, patch: Value) -> ConfigResult<Settings> {
        if !patch.is_object() {
            return Err(ConfigError::Invalid(
                "settings patch must be an object".into(),
            ));
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
    let mut value: Value = serde_json::from_str(text)?;
    migrate(&mut value);
    let mut settings: Settings = serde_json::from_value(value)?;
    settings.version = SETTINGS_VERSION;
    settings.validate()?;
    Ok(settings)
}

/// Brings an older file's JSON up to `SETTINGS_VERSION`, one step at a time.
fn migrate(value: &mut Value) {
    let version = value.get("version").and_then(Value::as_u64).unwrap_or(1);
    if version < 2 {
        // rowLimit (rows sent to the grid, default 1000) has no equivalent:
        // the grid now pages through results held in the backend.
        if let Some(query) = value.get_mut("query").and_then(Value::as_object_mut) {
            query.remove("rowLimit");
        }
    }
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
        fs::write(
            &path,
            r#"{ "version": 1, "connection": { "connectTimeoutSecs": 5 } }"#,
        )
        .unwrap();
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
            .update(json!({ "general": { "theme": "dark" }, "query": { "pageSize": 500 } }))
            .unwrap();
        let reloaded = store(&dir).get();
        assert_eq!(saved, reloaded);
        assert_eq!(reloaded.general.theme, Theme::Dark);
        assert_eq!(reloaded.query.page_size, 500);
    }

    #[test]
    fn version_1_file_is_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{ "version": 1, "general": { "theme": "dark" }, "query": { "rowLimit": 0, "queryTimeoutSecs": 30 } }"#,
        )
        .unwrap();
        let s = store(&dir).get();
        assert_eq!(s.version, SETTINGS_VERSION);
        assert_eq!(s.general.theme, Theme::Dark);
        assert_eq!(s.query.query_timeout_secs, 30);
        assert_eq!(
            s.query.max_rows_held,
            QuerySettings::default().max_rows_held
        );
        assert!(path.exists(), "a valid old file must not be moved to .bak");
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
        let err = s
            .update(json!({ "editor": { "fontSize": 1000 } }))
            .unwrap_err();
        assert!(matches!(err, ConfigError::Invalid(_)));
        let err = s
            .update(json!({ "editor": { "fontSize": "big" } }))
            .unwrap_err();
        assert!(matches!(err, ConfigError::Json(_)));
        assert_eq!(s.get(), Settings::default());
        assert!(!s.path().exists());
    }

    #[test]
    fn unknown_fields_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{ "version": 1, "bogus": 1, "editor": { "tabSize": 4 } }"#,
        )
        .unwrap();
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
        s.update(json!({ "general": { "theme": "light" } }))
            .unwrap();
        assert_eq!(s.reset().unwrap(), Settings::default());
        assert_eq!(store(&dir).get(), Settings::default());
    }
}
