use std::{collections::HashMap, fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

/// Everything a driver needs to open a connection.
/// Target, auth and transport are independent, so each is its own enum.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ConnectConfig {
    pub target: Target,
    pub auth: Auth,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub tunnel: Option<SshTunnel>,
    #[serde(default)]
    pub options: HashMap<String, String>, // driver extras: application_name, search_path…
}

// ---------- where the database is ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "method", rename_all = "camelCase")]
pub enum Target {
    Url {
        url: String, // postgres://…, mysql://…; drivers normalize it into Host + Auth
    },
    Host {
        host: String,
        port: Option<u16>, // None = driver default (PG: 5432)
        database: Option<String>,
    },
    Socket {
        path: PathBuf, // PG / MySQL local unix socket
        database: Option<String>,
    },
    File {
        path: PathBuf, // SQLite, DuckDB
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    Url,
    Host,
    Socket,
    File,
}

impl Target {
    pub fn kind(&self) -> TargetKind {
        match self {
            Target::Url { .. } => TargetKind::Url,
            Target::Host { .. } => TargetKind::Host,
            Target::Socket { .. } => TargetKind::Socket,
            Target::File { .. } => TargetKind::File,
        }
    }
}

// ---------- how you prove who you are ----------

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Auth {
    None,
    Password {
        user: String,
        password: Secret,
    },
    PasswordPrompt {
        user: String, // ask every time, never stored
    },
    PgPass {
        user: String, // ~/.pgpass / PGPASSWORD
    },
    ClientCert {
        user: String,
        cert: PathBuf,
        key: PathBuf,
    },
    Token {
        user: String,
        provider: TokenProvider, // cloud IAM; later
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum AuthKind {
    None,
    Password,
    PasswordPrompt,
    PgPass,
    ClientCert,
    Token,
}

impl Auth {
    pub fn kind(&self) -> AuthKind {
        match self {
            Auth::None => AuthKind::None,
            Auth::Password { .. } => AuthKind::Password,
            Auth::PasswordPrompt { .. } => AuthKind::PasswordPrompt,
            Auth::PgPass { .. } => AuthKind::PgPass,
            Auth::ClientCert { .. } => AuthKind::ClientCert,
            Auth::Token { .. } => AuthKind::Token,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "provider", rename_all = "camelCase")]
pub enum TokenProvider {
    AwsIam { region: String },
    AzureAd,
}

// ---------- how you reach it ----------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    #[serde(default)]
    pub mode: SslMode,
    pub ca_cert: Option<PathBuf>, // custom root CA for VerifyCa / VerifyFull
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum SslMode {
    Disable,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SshTunnel {
    pub host: String,
    pub port: Option<u16>, // None = 22
    pub user: String,
    pub auth: SshAuth,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SshAuth {
    Password { password: Secret },
    Key { path: PathBuf, passphrase: Option<Secret> },
    Agent,
}

// ---------- secrets ----------

/// Password or passphrase. Can be received from the frontend but is never
/// serialized back, and prints as `***` in logs.
#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

// ---------- driver-specific extra options ----------

/// Describes one entry of `ConnectConfig::options` so the UI can render it.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FieldSpec {
    pub key: &'static str,   // "application_name"
    pub label: &'static str, // "Application name"
    pub kind: FieldKind,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FieldKind {
    Text,
    Number,
    Password,
    Bool,
    File,
    Select { options: Vec<&'static str> },
}
