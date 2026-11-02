use serde::{Serialize, Serializer, ser::SerializeStruct};

pub type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
}

impl ConfigError {
    pub fn kind(&self) -> &'static str {
        match self {
            ConfigError::Io(_) => "io",
            ConfigError::Json(_) => "json",
            ConfigError::Sqlite(_) => "sqlite",
            ConfigError::Keyring(_) => "keyring",
            ConfigError::NotFound(_) => "notFound",
            ConfigError::Invalid(_) => "invalid",
        }
    }
}

impl From<keyring::Error> for ConfigError {
    fn from(err: keyring::Error) -> Self {
        ConfigError::Keyring(err.to_string())
    }
}

/// Serialized as `{ kind, message }` so Tauri commands can return it as is.
impl Serialize for ConfigError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("ConfigError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

/// TS shape of the custom `Serialize` above.
#[cfg(feature = "specta")]
impl specta::Type for ConfigError {
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        shape::ConfigError::definition(types)
    }
}

#[cfg(feature = "specta")]
mod shape {
    #![allow(dead_code)]
    use serde::Serialize;

    #[derive(Serialize, specta::Type)]
    pub struct ConfigError {
        kind: ConfigErrorKind,
        message: String,
    }

    /// Must match `ConfigError::kind`.
    #[derive(Serialize, specta::Type)]
    #[serde(rename_all = "camelCase")]
    pub enum ConfigErrorKind {
        Io,
        Json,
        Sqlite,
        Keyring,
        NotFound,
        Invalid,
    }
}
