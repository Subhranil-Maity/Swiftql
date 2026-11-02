use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use driver_core::connect::{
    Auth, ConnectConfig, Secret, SshAuth, SshTunnel, Target, TlsConfig, TokenProvider,
};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::Db,
    error::{ConfigError, ConfigResult},
    secrets::{SecretKind, SecretRef, SecretStore},
};

// ---------- what the frontend sees ----------

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SavedConnection {
    pub id: Uuid,
    pub name: String,
    pub driver: String,
    pub folder: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub target: Target,
    pub auth: StoredAuth,
    pub tls: TlsConfig,
    pub tunnel: Option<StoredTunnel>,
    pub options: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub last_status: Option<ConnectionStatus>,
}

/// `Auth` without secret values; `saved` says whether the keyring has one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum StoredAuth {
    None,
    Password { user: String, saved: bool },
    PasswordPrompt { user: String },
    PgPass { user: String },
    ClientCert { user: String, cert: PathBuf, key: PathBuf },
    Token { user: String, provider: TokenProvider },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct StoredTunnel {
    pub host: String,
    pub port: Option<u16>,
    pub user: String,
    pub auth: StoredSshAuth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum StoredSshAuth {
    Password { saved: bool },
    Key { path: PathBuf, passphrase_saved: bool },
    Agent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub at: DateTime<Utc>,
    pub ok: bool,
    pub error_kind: Option<String>, // driver_core::error::ErrorKind, camelCase
    pub message: Option<String>,
    pub server_version: Option<String>,
}

// ---------- what the frontend sends ----------

/// A connection as entered in the UI, secrets included.
///
/// With `save_password`, an empty password (or passphrase) keeps the secret
/// already in the keyring, so the UI does not need to know it to save edits.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub name: String,
    pub driver: String,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    pub target: Target,
    pub auth: Auth,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub tunnel: Option<SshTunnel>,
    #[serde(default)]
    pub options: HashMap<String, String>,
    #[serde(default)]
    pub save_password: bool,
}

/// Everything in `connections.config_json`. The table columns only exist for
/// listing and sorting.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredConfig {
    target: Target,
    auth: StoredAuth,
    #[serde(default)]
    tls: TlsConfig,
    #[serde(default)]
    tunnel: Option<StoredTunnel>,
    #[serde(default)]
    options: HashMap<String, String>,
}

enum SecretAction {
    Set(Secret),
    Keep,
    Delete,
}

impl SecretAction {
    /// Set when there is a value, keep the existing one when it is empty.
    fn from_input(save: bool, value: &Secret) -> Self {
        match (save, value.expose().is_empty()) {
            (false, _) => SecretAction::Delete,
            (true, true) => SecretAction::Keep,
            (true, false) => SecretAction::Set(value.clone()),
        }
    }
}

// ---------- repository ----------

const SELECT: &str = "
    SELECT c.id, c.name, c.driver, c.folder, c.color, c.sort_order, c.config_json,
           c.created_at, c.updated_at, c.last_used_at,
           s.at, s.ok, s.error_kind, s.message, s.server_version
    FROM connections c
    LEFT JOIN connection_status s ON s.connection_id = c.id";

pub struct ConnectionRepo<'a> {
    db: &'a Db,
    secrets: &'a SecretStore,
}

impl<'a> ConnectionRepo<'a> {
    pub(crate) fn new(db: &'a Db, secrets: &'a SecretStore) -> Self {
        Self { db, secrets }
    }

    pub fn list(&self) -> ConfigResult<Vec<SavedConnection>> {
        let conn = self.db.lock();
        query(
            &conn,
            &format!(
                "{SELECT} ORDER BY COALESCE(c.folder, ''), c.sort_order, c.name COLLATE NOCASE"
            ),
            [],
        )
    }

    pub fn recent(&self, limit: u32) -> ConfigResult<Vec<SavedConnection>> {
        let conn = self.db.lock();
        query(
            &conn,
            &format!(
                "{SELECT} WHERE c.last_used_at IS NOT NULL ORDER BY c.last_used_at DESC LIMIT ?1"
            ),
            [limit],
        )
    }

    pub fn get(&self, id: Uuid) -> ConfigResult<SavedConnection> {
        get(&self.db.lock(), id)
    }

    /// Inserts or updates a connection and syncs its secrets with the keyring.
    pub fn save(&self, input: ConnectionInput) -> ConfigResult<SavedConnection> {
        let name = input.name.trim().to_owned();
        if name.is_empty() {
            return Err(ConfigError::Invalid("connection name must not be empty".into()));
        }
        if input.driver.trim().is_empty() {
            return Err(ConfigError::Invalid("connection driver must not be empty".into()));
        }

        let conn = self.db.lock();
        let id = input.id.unwrap_or_else(Uuid::new_v4);
        let is_new = find(&conn, id)?.is_none();

        let auth = self.store_auth(id, input.auth, input.save_password)?;
        let tunnel = self.store_tunnel(id, input.tunnel, input.save_password)?;
        let config = StoredConfig {
            target: input.target,
            auth,
            tls: input.tls,
            tunnel,
            options: input.options,
        };

        let now = Utc::now().timestamp_millis();
        let written = conn.execute(
            "INSERT INTO connections
                 (id, name, driver, folder, color, sort_order, config_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5,
                     (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM connections), ?6, ?7, ?7)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 driver = excluded.driver,
                 folder = excluded.folder,
                 color = excluded.color,
                 config_json = excluded.config_json,
                 updated_at = excluded.updated_at",
            params![
                id.to_string(),
                name,
                input.driver,
                blank_to_none(input.folder),
                blank_to_none(input.color),
                serde_json::to_string(&config)?,
                now,
            ],
        );
        if let Err(err) = written {
            if is_new {
                // Don't leave secrets behind for a connection that was never created.
                let _ = self.secrets.delete_all(id);
            }
            return Err(err.into());
        }
        get(&conn, id)
    }

    /// Deletes the connection, its status and all of its secrets.
    pub fn delete(&self, id: Uuid) -> ConfigResult<()> {
        let conn = self.db.lock();
        self.secrets.delete_all(id)?;
        let deleted = conn.execute("DELETE FROM connections WHERE id = ?1", [id.to_string()])?;
        if deleted == 0 {
            return Err(not_found(id));
        }
        Ok(())
    }

    pub fn touch(&self, id: Uuid) -> ConfigResult<()> {
        let updated = self.db.lock().execute(
            "UPDATE connections SET last_used_at = ?2 WHERE id = ?1",
            params![id.to_string(), Utc::now().timestamp_millis()],
        )?;
        if updated == 0 {
            return Err(not_found(id));
        }
        Ok(())
    }

    pub fn set_status(&self, id: Uuid, status: &ConnectionStatus) -> ConfigResult<()> {
        let conn = self.db.lock();
        if find(&conn, id)?.is_none() {
            return Err(not_found(id));
        }
        conn.execute(
            "INSERT OR REPLACE INTO connection_status
                 (connection_id, at, ok, error_kind, message, server_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id.to_string(),
                status.at.timestamp_millis(),
                status.ok,
                status.error_kind,
                status.message,
                status.server_version,
            ],
        )?;
        Ok(())
    }

    /// Forgets one secret and clears its `saved` flag.
    pub fn clear_secret(&self, id: Uuid, kind: SecretKind) -> ConfigResult<SavedConnection> {
        let conn = self.db.lock();
        let json = find(&conn, id)?.ok_or_else(|| not_found(id))?;
        let mut config: StoredConfig = serde_json::from_str(&json)?;
        self.secrets.delete(SecretRef::new(id, kind))?;

        match (kind, &mut config.auth, config.tunnel.as_mut().map(|t| &mut t.auth)) {
            (SecretKind::DbPassword, StoredAuth::Password { saved, .. }, _) => *saved = false,
            (SecretKind::SshPassword, _, Some(StoredSshAuth::Password { saved })) => {
                *saved = false
            }
            (SecretKind::SshPassphrase, _, Some(StoredSshAuth::Key { passphrase_saved, .. })) => {
                *passphrase_saved = false
            }
            _ => {}
        }
        conn.execute(
            "UPDATE connections SET config_json = ?2 WHERE id = ?1",
            params![id.to_string(), serde_json::to_string(&config)?],
        )?;
        get(&conn, id)
    }

    /// Rebuilds a full `ConnectConfig` with secrets from the keyring.
    ///
    /// `prompt` is a password the user just typed and wins over a saved one.
    /// With neither, the auth comes back as `Auth::PasswordPrompt` so the app
    /// knows to ask.
    pub fn resolve(&self, id: Uuid, prompt: Option<Secret>) -> ConfigResult<ConnectConfig> {
        let json = find(&self.db.lock(), id)?.ok_or_else(|| not_found(id))?;
        let config: StoredConfig = serde_json::from_str(&json)?;

        let auth = match config.auth {
            StoredAuth::None => Auth::None,
            StoredAuth::Password { user, saved } => {
                let password = match prompt {
                    Some(p) => Some(p),
                    None if saved => self.secrets.get(SecretRef::new(id, SecretKind::DbPassword))?,
                    None => None,
                };
                match password {
                    Some(password) => Auth::Password { user, password },
                    None => Auth::PasswordPrompt { user },
                }
            }
            StoredAuth::PasswordPrompt { user } => match prompt {
                Some(password) => Auth::Password { user, password },
                None => Auth::PasswordPrompt { user },
            },
            StoredAuth::PgPass { user } => Auth::PgPass { user },
            StoredAuth::ClientCert { user, cert, key } => Auth::ClientCert { user, cert, key },
            StoredAuth::Token { user, provider } => Auth::Token { user, provider },
        };

        let tunnel = match config.tunnel {
            None => None,
            Some(t) => {
                let auth = match t.auth {
                    StoredSshAuth::Agent => SshAuth::Agent,
                    StoredSshAuth::Password { .. } => {
                        // TODO: SshAuth has no prompt variant yet, so a missing SSH password is an error
                        let password = self
                            .secrets
                            .get(SecretRef::new(id, SecretKind::SshPassword))?
                            .ok_or_else(|| {
                                ConfigError::NotFound(
                                    "SSH password is not saved for this connection".into(),
                                )
                            })?;
                        SshAuth::Password { password }
                    }
                    StoredSshAuth::Key {
                        path,
                        passphrase_saved,
                    } => SshAuth::Key {
                        path,
                        passphrase: if passphrase_saved {
                            self.secrets.get(SecretRef::new(id, SecretKind::SshPassphrase))?
                        } else {
                            None
                        },
                    },
                };
                Some(SshTunnel {
                    host: t.host,
                    port: t.port,
                    user: t.user,
                    auth,
                })
            }
        };

        Ok(ConnectConfig {
            target: config.target,
            auth,
            tls: config.tls,
            tunnel,
            options: config.options,
        })
    }

    /// Turns form input into a `ConnectConfig` without saving anything, e.g.
    /// to test a connection.
    ///
    /// When `input.id` is an existing connection, an empty password or
    /// passphrase is filled from the keyring, like `save` keeps it on edit.
    /// The UI never gets saved secrets back, so this lets you test an edited
    /// connection without retyping its password.
    pub fn resolve_input(&self, input: ConnectionInput) -> ConfigResult<ConnectConfig> {
        let saved = |kind: SecretKind| -> ConfigResult<Option<Secret>> {
            match input.id {
                Some(id) => self.secrets.get(SecretRef::new(id, kind)),
                None => Ok(None),
            }
        };
        let fill = |value: Secret, kind: SecretKind| -> ConfigResult<Secret> {
            if !value.expose().is_empty() {
                return Ok(value);
            }
            Ok(saved(kind)?.unwrap_or(value))
        };

        let auth = match input.auth {
            Auth::Password { user, password } => Auth::Password {
                user,
                password: fill(password, SecretKind::DbPassword)?,
            },
            other => other,
        };
        let tunnel = match input.tunnel {
            None => None,
            Some(t) => {
                let auth = match t.auth {
                    SshAuth::Password { password } => SshAuth::Password {
                        password: fill(password, SecretKind::SshPassword)?,
                    },
                    SshAuth::Key { path, passphrase } => SshAuth::Key {
                        path,
                        passphrase: match passphrase {
                            Some(p) => Some(fill(p, SecretKind::SshPassphrase)?),
                            None => saved(SecretKind::SshPassphrase)?,
                        },
                    },
                    SshAuth::Agent => SshAuth::Agent,
                };
                Some(SshTunnel { auth, ..t })
            }
        };

        Ok(ConnectConfig {
            target: input.target,
            auth,
            tls: input.tls,
            tunnel,
            options: input.options,
        })
    }

    fn store_auth(&self, id: Uuid, auth: Auth, save: bool) -> ConfigResult<StoredAuth> {
        let db_password = SecretRef::new(id, SecretKind::DbPassword);
        Ok(match auth {
            Auth::Password { user, password } => {
                let saved = self.apply(db_password, SecretAction::from_input(save, &password))?;
                StoredAuth::Password { user, saved }
            }
            other => {
                self.secrets.delete(db_password)?;
                match other {
                    Auth::None => StoredAuth::None,
                    Auth::PasswordPrompt { user } => StoredAuth::PasswordPrompt { user },
                    Auth::PgPass { user } => StoredAuth::PgPass { user },
                    Auth::ClientCert { user, cert, key } => StoredAuth::ClientCert { user, cert, key },
                    Auth::Token { user, provider } => StoredAuth::Token { user, provider },
                    Auth::Password { .. } => unreachable!(),
                }
            }
        })
    }

    fn store_tunnel(
        &self,
        id: Uuid,
        tunnel: Option<SshTunnel>,
        save: bool,
    ) -> ConfigResult<Option<StoredTunnel>> {
        let password = SecretRef::new(id, SecretKind::SshPassword);
        let passphrase = SecretRef::new(id, SecretKind::SshPassphrase);
        let Some(t) = tunnel else {
            self.secrets.delete(password)?;
            self.secrets.delete(passphrase)?;
            return Ok(None);
        };
        let auth = match t.auth {
            SshAuth::Password { password: value } => {
                self.secrets.delete(passphrase)?;
                let saved = self.apply(password, SecretAction::from_input(save, &value))?;
                StoredSshAuth::Password { saved }
            }
            SshAuth::Key {
                path,
                passphrase: value,
            } => {
                self.secrets.delete(password)?;
                let action = match value {
                    Some(value) => SecretAction::from_input(save, &value),
                    None => SecretAction::Delete,
                };
                let passphrase_saved = self.apply(passphrase, action)?;
                StoredSshAuth::Key {
                    path,
                    passphrase_saved,
                }
            }
            SshAuth::Agent => {
                self.secrets.delete(password)?;
                self.secrets.delete(passphrase)?;
                StoredSshAuth::Agent
            }
        };
        Ok(Some(StoredTunnel {
            host: t.host,
            port: t.port,
            user: t.user,
            auth,
        }))
    }

    /// Returns whether the secret is saved afterwards.
    fn apply(&self, r: SecretRef, action: SecretAction) -> ConfigResult<bool> {
        match action {
            SecretAction::Set(value) => self.secrets.set(r, &value).map(|_| true),
            SecretAction::Keep => self.secrets.exists(r),
            SecretAction::Delete => self.secrets.delete(r).map(|_| false),
        }
    }
}

// ---------- row helpers ----------

fn find(conn: &Connection, id: Uuid) -> ConfigResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT config_json FROM connections WHERE id = ?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?)
}

fn get(conn: &Connection, id: Uuid) -> ConfigResult<SavedConnection> {
    query(conn, &format!("{SELECT} WHERE c.id = ?1"), [id.to_string()])?
        .pop()
        .ok_or_else(|| not_found(id))
}

fn query(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> ConfigResult<Vec<SavedConnection>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, RawRow::read)?;
    rows.map(|row| row?.into_saved()).collect()
}

struct RawRow {
    id: String,
    name: String,
    driver: String,
    folder: Option<String>,
    color: Option<String>,
    sort_order: i64,
    config_json: String,
    created_at: i64,
    updated_at: i64,
    last_used_at: Option<i64>,
    status_at: Option<i64>,
    status_ok: Option<bool>,
    error_kind: Option<String>,
    message: Option<String>,
    server_version: Option<String>,
}

impl RawRow {
    fn read(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get(0)?,
            name: r.get(1)?,
            driver: r.get(2)?,
            folder: r.get(3)?,
            color: r.get(4)?,
            sort_order: r.get(5)?,
            config_json: r.get(6)?,
            created_at: r.get(7)?,
            updated_at: r.get(8)?,
            last_used_at: r.get(9)?,
            status_at: r.get(10)?,
            status_ok: r.get(11)?,
            error_kind: r.get(12)?,
            message: r.get(13)?,
            server_version: r.get(14)?,
        })
    }

    fn into_saved(self) -> ConfigResult<SavedConnection> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| ConfigError::Invalid(format!("bad connection id {:?}: {e}", self.id)))?;
        let config: StoredConfig = serde_json::from_str(&self.config_json)?;
        let last_status = match (self.status_at, self.status_ok) {
            (Some(at), Some(ok)) => Some(ConnectionStatus {
                at: from_millis(at),
                ok,
                error_kind: self.error_kind,
                message: self.message,
                server_version: self.server_version,
            }),
            _ => None,
        };
        Ok(SavedConnection {
            id,
            name: self.name,
            driver: self.driver,
            folder: self.folder,
            color: self.color,
            sort_order: self.sort_order,
            target: config.target,
            auth: config.auth,
            tls: config.tls,
            tunnel: config.tunnel,
            options: config.options,
            created_at: from_millis(self.created_at),
            updated_at: from_millis(self.updated_at),
            last_used_at: self.last_used_at.map(from_millis),
            last_status,
        })
    }
}

fn from_millis(ms: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(ms).unwrap_or(DateTime::UNIX_EPOCH)
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

fn not_found(id: Uuid) -> ConfigError {
    ConfigError::NotFound(format!("connection {id} not found"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::{Value, json};

    use super::*;
    use crate::secrets::MemoryBackend;

    struct Fixture {
        db: Db,
        backend: Arc<MemoryBackend>,
        secrets: SecretStore,
    }

    impl Fixture {
        fn new() -> Self {
            let backend = Arc::new(MemoryBackend::new());
            Self {
                db: Db::open_in_memory().unwrap(),
                secrets: SecretStore::new(backend.clone()),
                backend,
            }
        }

        fn repo(&self) -> ConnectionRepo<'_> {
            ConnectionRepo::new(&self.db, &self.secrets)
        }

        fn secret(&self, id: Uuid, kind: SecretKind) -> Option<String> {
            self.secrets
                .get(SecretRef::new(id, kind))
                .unwrap()
                .map(|s| s.expose().to_owned())
        }
    }

    fn input(value: Value) -> ConnectionInput {
        serde_json::from_value(value).unwrap()
    }

    fn pg(name: &str, password: &str, save: bool) -> ConnectionInput {
        input(json!({
            "name": name,
            "driver": "postgres",
            "target": { "method": "host", "host": "localhost", "port": 5432, "database": "app" },
            "auth": { "kind": "password", "user": "alice", "password": password },
            "savePassword": save,
        }))
    }

    #[test]
    fn crud() {
        let f = Fixture::new();
        let repo = f.repo();

        let a = repo.save(pg("Beta", "pw", true)).unwrap();
        let b = repo.save(pg("alpha", "pw", true)).unwrap();
        assert_eq!(repo.get(a.id).unwrap().name, "Beta");
        let names: Vec<_> = repo.list().unwrap().into_iter().map(|c| c.name).collect();
        assert_eq!(names, ["Beta", "alpha"]); // sort_order first

        let mut edit = pg("Renamed", "", true);
        edit.id = Some(a.id);
        edit.folder = Some("prod".into());
        let edited = repo.save(edit).unwrap();
        assert_eq!(edited.name, "Renamed");
        assert_eq!(edited.created_at, a.created_at);
        assert_eq!(edited.sort_order, a.sort_order);
        assert_eq!(repo.list().unwrap().len(), 2);

        repo.delete(b.id).unwrap();
        assert!(matches!(repo.get(b.id), Err(ConfigError::NotFound(_))));
        assert!(matches!(repo.delete(b.id), Err(ConfigError::NotFound(_))));
        assert_eq!(repo.list().unwrap().len(), 1);
    }

    #[test]
    fn rejects_empty_name() {
        let f = Fixture::new();
        assert!(matches!(
            f.repo().save(pg("  ", "pw", true)),
            Err(ConfigError::Invalid(_))
        ));
    }

    #[test]
    fn saved_password_goes_to_keyring_not_db() {
        let f = Fixture::new();
        let c = f.repo().save(pg("db", "s3cret", true)).unwrap();
        assert!(matches!(c.auth, StoredAuth::Password { saved: true, .. }));
        assert_eq!(f.secret(c.id, SecretKind::DbPassword).as_deref(), Some("s3cret"));

        let json: String = f
            .db
            .lock()
            .query_row("SELECT config_json FROM connections", [], |r| r.get(0))
            .unwrap();
        assert!(!json.contains("s3cret"));
        assert!(!serde_json::to_string(&c).unwrap().contains("s3cret"));
    }

    #[test]
    fn empty_password_keeps_existing_secret() {
        let f = Fixture::new();
        let repo = f.repo();
        let c = repo.save(pg("db", "s3cret", true)).unwrap();
        let mut edit = pg("db", "", true);
        edit.id = Some(c.id);
        let c = repo.save(edit).unwrap();
        assert!(matches!(c.auth, StoredAuth::Password { saved: true, .. }));
        assert_eq!(f.secret(c.id, SecretKind::DbPassword).as_deref(), Some("s3cret"));
    }

    #[test]
    fn save_password_false_clears_secret() {
        let f = Fixture::new();
        let repo = f.repo();
        let c = repo.save(pg("db", "s3cret", true)).unwrap();
        let mut edit = pg("db", "s3cret", false);
        edit.id = Some(c.id);
        let c = repo.save(edit).unwrap();
        assert!(matches!(c.auth, StoredAuth::Password { saved: false, .. }));
        assert!(f.backend.is_empty());
    }

    #[test]
    fn switching_auth_kind_clears_secret() {
        let f = Fixture::new();
        let repo = f.repo();
        let c = repo.save(pg("db", "s3cret", true)).unwrap();
        let mut edit = pg("db", "", true);
        edit.id = Some(c.id);
        edit.auth = Auth::PasswordPrompt { user: "alice".into() };
        repo.save(edit).unwrap();
        assert!(f.backend.is_empty());
    }

    #[test]
    fn delete_removes_all_secrets() {
        let f = Fixture::new();
        let repo = f.repo();
        let mut i = pg("db", "s3cret", true);
        i.tunnel = Some(SshTunnel {
            host: "bastion".into(),
            port: None,
            user: "ops".into(),
            auth: SshAuth::Key {
                path: "/home/ops/.ssh/id_ed25519".into(),
                passphrase: Some(Secret::new("pp")),
            },
        });
        let c = repo.save(i).unwrap();
        assert!(matches!(
            c.tunnel.as_ref().unwrap().auth,
            StoredSshAuth::Key { passphrase_saved: true, .. }
        ));
        assert_eq!(f.backend.len(), 2);
        repo.delete(c.id).unwrap();
        assert!(f.backend.is_empty());
    }

    #[test]
    fn resolve_with_saved_password() {
        let f = Fixture::new();
        let repo = f.repo();
        let c = repo.save(pg("db", "s3cret", true)).unwrap();
        let config = repo.resolve(c.id, None).unwrap();
        assert!(matches!(config.target, Target::Host { ref host, .. } if host == "localhost"));
        match config.auth {
            Auth::Password { user, password } => {
                assert_eq!(user, "alice");
                assert_eq!(password.expose(), "s3cret");
            }
            other => panic!("unexpected auth {other:?}"),
        }
        // A typed password wins over the saved one.
        let config = repo.resolve(c.id, Some(Secret::new("typed"))).unwrap();
        assert!(matches!(config.auth, Auth::Password { ref password, .. } if password.expose() == "typed"));
    }

    #[test]
    fn resolve_without_saved_password() {
        let f = Fixture::new();
        let repo = f.repo();
        let c = repo.save(pg("db", "s3cret", false)).unwrap();
        let config = repo.resolve(c.id, None).unwrap();
        assert!(matches!(config.auth, Auth::PasswordPrompt { ref user } if user == "alice"));
        let config = repo.resolve(c.id, Some(Secret::new("typed"))).unwrap();
        assert!(matches!(config.auth, Auth::Password { ref password, .. } if password.expose() == "typed"));
    }

    fn password_of(config: &ConnectConfig) -> &str {
        match &config.auth {
            Auth::Password { password, .. } => password.expose(),
            other => panic!("unexpected auth {other:?}"),
        }
    }

    #[test]
    fn resolve_input_fills_empty_password_from_keyring() {
        let f = Fixture::new();
        let repo = f.repo();
        let c = repo.save(pg("db", "s3cret", true)).unwrap();

        // Editing an existing connection: empty password → saved one.
        let mut edit = pg("db", "", true);
        edit.id = Some(c.id);
        assert_eq!(password_of(&repo.resolve_input(edit.clone()).unwrap()), "s3cret");

        // A typed password wins.
        let mut typed = pg("db", "typed", true);
        typed.id = Some(c.id);
        assert_eq!(password_of(&repo.resolve_input(typed).unwrap()), "typed");

        // A new connection has nothing to fill from.
        let fresh = pg("db", "", true);
        assert_eq!(password_of(&repo.resolve_input(fresh).unwrap()), "");

        // Nothing is written.
        assert_eq!(repo.list().unwrap().len(), 1);
        assert_eq!(f.backend.len(), 1);
    }

    #[test]
    fn resolve_input_keeps_target_and_other_auth() {
        let f = Fixture::new();
        let mut i = pg("db", "", false);
        i.auth = Auth::PasswordPrompt { user: "bob".into() };
        let config = f.repo().resolve_input(i).unwrap();
        assert!(matches!(config.auth, Auth::PasswordPrompt { ref user } if user == "bob"));
        assert!(matches!(config.target, Target::Host { port: Some(5432), .. }));
    }

    #[test]
    fn clear_secret_updates_flag() {
        let f = Fixture::new();
        let repo = f.repo();
        let c = repo.save(pg("db", "s3cret", true)).unwrap();
        let c = repo.clear_secret(c.id, SecretKind::DbPassword).unwrap();
        assert!(matches!(c.auth, StoredAuth::Password { saved: false, .. }));
        assert!(f.backend.is_empty());
    }

    #[test]
    fn touch_status_and_recent() {
        let f = Fixture::new();
        let repo = f.repo();
        let a = repo.save(pg("a", "", false)).unwrap();
        let b = repo.save(pg("b", "", false)).unwrap();
        assert!(repo.recent(10).unwrap().is_empty());

        repo.touch(b.id).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        repo.touch(a.id).unwrap();
        let recent: Vec<_> = repo.recent(10).unwrap().into_iter().map(|c| c.name).collect();
        assert_eq!(recent, ["a", "b"]);
        assert_eq!(repo.recent(1).unwrap().len(), 1);

        let status = ConnectionStatus {
            at: from_millis(Utc::now().timestamp_millis()),
            ok: false,
            error_kind: Some("auth".into()),
            message: Some("password authentication failed".into()),
            server_version: None,
        };
        repo.set_status(a.id, &status).unwrap();
        assert_eq!(repo.get(a.id).unwrap().last_status, Some(status.clone()));
        assert!(repo.get(b.id).unwrap().last_status.is_none());
        assert!(matches!(
            repo.set_status(Uuid::new_v4(), &status),
            Err(ConfigError::NotFound(_))
        ));

        repo.delete(a.id).unwrap();
        let rows: i64 = f
            .db
            .lock()
            .query_row("SELECT COUNT(*) FROM connection_status", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }
}
