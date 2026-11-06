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
    /// From the app settings, not the saved connection.
    #[serde(default)]
    pub timeouts: Timeouts,
    /// How SSH host keys are checked; set by the app before connecting.
    #[serde(skip)]
    pub host_keys: Option<HostKeyCheck>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Timeouts {
    /// Seconds to reach the server (and the SSH host). None = driver default.
    pub connect_secs: Option<u32>,
    /// TCP / SSH keepalive interval in seconds. None or 0 = off.
    pub keepalive_secs: Option<u32>,
}

/// SSH host key checking: which files list known hosts, and what to do with
/// a host that isn't in them. A key that differs from a known one is always
/// refused.
#[derive(Debug, Clone)]
pub struct HostKeyCheck {
    pub policy: HostKeyPolicy,
    /// Read only: the user's `~/.ssh/known_hosts`.
    pub user_known_hosts: Option<PathBuf>,
    /// Read and written: hosts trusted in SwiftQL.
    pub app_known_hosts: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum HostKeyPolicy {
    /// Fail with `SSH_HOST_UNKNOWN` so the user can look at the fingerprint.
    #[default]
    Ask,
    /// Trust and remember it.
    AcceptNew,
    /// Refuse it.
    Strict,
}

/// `DbError::code` values for SSH problems the UI can act on.
pub mod ssh_codes {
    /// The host key isn't known; `detail` is JSON: host, port, keyType,
    /// fingerprint, key (OpenSSH public key line).
    pub const HOST_UNKNOWN: &str = "SSH_HOST_UNKNOWN";
    /// The host key differs from the known one.
    pub const HOST_CHANGED: &str = "SSH_HOST_CHANGED";
    pub const PASSWORD_REQUIRED: &str = "SSH_PASSWORD_REQUIRED";
    pub const PASSPHRASE_REQUIRED: &str = "SSH_PASSPHRASE_REQUIRED";
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
    /// Where host, port, user and keys come from.
    #[serde(default)]
    pub source: SshSource,
    /// With `SshSource::Config`, an empty host / user / port uses the config's.
    pub host: String,
    pub port: Option<u16>, // None = 22
    pub user: String,
    pub auth: SshAuth,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SshSource {
    /// Host, port, user and auth as entered.
    #[default]
    Manual,
    /// A `Host` entry of `~/.ssh/config`, resolved when connecting so later
    /// edits apply: HostName, User, Port, IdentityFile, ProxyJump.
    Config { alias: String },
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SshAuth {
    Password {
        password: Secret,
    },
    /// A password that isn't saved; the app asks for it.
    PasswordPrompt,
    Key {
        path: PathBuf,
        passphrase: Option<Secret>,
    },
    Agent,
    /// IdentityFile keys from `~/.ssh/config`, then the default keys
    /// (`~/.ssh/id_ed25519`, ...), then the agent, as `ssh` does.
    Auto,
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
