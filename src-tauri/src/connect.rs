//! Commands that talk to a database: test, health check, connect, disconnect.

use std::sync::Arc;

use chrono::Utc;
use config_store::{ConfigError, ConfigManager, ConnectionInput, ConnectionStatus};
use driver_core::{
    connect::{Auth, ConnectConfig, Secret},
    connection::ServerVersion,
    driver::Driver,
    error::{DbError, ErrorKind},
};
use serde::Serialize;
use specta::Type;
use tauri::State;
use uuid::Uuid;

use crate::{
    config::Config,
    drivers::DriverRegistry,
    session::{Session, SessionInfo, SessionManager},
};

/// `DbError::code` for a saved connection that has no password to use.
pub const PASSWORD_REQUIRED: &str = "PASSWORD_REQUIRED";

fn from_config(err: ConfigError) -> DbError {
    match err {
        ConfigError::NotFound(msg) => DbError::new(ErrorKind::NotFound, msg),
        other => DbError::new(ErrorKind::Internal, other.to_string()),
    }
}

/// Keyring and SQLite calls block, so they run on the blocking pool.
async fn blocking<T, F>(config: &Config, f: F) -> Result<T, DbError>
where
    T: Send + 'static,
    F: FnOnce(&ConfigManager) -> Result<T, ConfigError> + Send + 'static,
{
    let config = Arc::clone(config);
    tauri::async_runtime::spawn_blocking(move || f(&config))
        .await
        .map_err(|e| DbError::new(ErrorKind::Internal, e.to_string()))?
        .map_err(from_config)
}

fn driver(registry: &DriverRegistry, id: &str) -> Result<Arc<dyn Driver>, DbError> {
    registry.get(id).ok_or_else(|| {
        DbError::new(
            ErrorKind::Unsupported,
            format!("no driver named {id:?} is available"),
        )
    })
}

/// "16.4", or "8.0.36" when there is a patch number.
fn short_version(v: &ServerVersion) -> String {
    if v.patch > 0 {
        format!("{}.{}.{}", v.major, v.minor, v.patch)
    } else {
        format!("{}.{}", v.major, v.minor)
    }
}

/// Opens a connection from unsaved form input, reads the server version and
/// closes it again.
#[tauri::command]
#[specta::specta]
pub async fn test_connection(
    config: State<'_, Config>,
    registry: State<'_, DriverRegistry>,
    input: ConnectionInput,
) -> Result<ServerVersion, DbError> {
    let driver = driver(&registry, &input.driver)?;
    let connect_config = blocking(&config, move |c| c.connections().resolve_input(input)).await?;
    if let Auth::PasswordPrompt { .. } = connect_config.auth {
        // TODO: let the form ask for a one-off password to test with
        return Err(password_required()
            .with_hint("To test, switch Authentication to Password and enter it."));
    }
    let conn = driver.connect(&connect_config).await?;
    let version = conn.server_version().clone();
    conn.close().await;
    Ok(version)
}

/// What a health check found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HealthState {
    /// Connected and closed again.
    Online,
    /// The server answered but refused us: wrong user or password, missing
    /// database, no permission.
    Misconfigured,
    /// The server couldn't be reached in time.
    Offline,
    /// Needs a password that isn't saved, so it wasn't checked.
    NeedsPassword,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionHealth {
    pub state: HealthState,
    pub message: Option<String>,
    pub server_version: Option<ServerVersion>,
}

impl ConnectionHealth {
    fn from_error(err: &DbError) -> Self {
        let state = match err.kind {
            ErrorKind::Network | ErrorKind::Timeout => HealthState::Offline,
            _ => HealthState::Misconfigured,
        };
        Self {
            state,
            message: Some(err.message.clone()),
            server_version: None,
        }
    }
}

/// Connects to a saved connection and closes it again, to show whether it
/// works. Nothing is recorded: the result is only for the current screen.
///
/// Fails only when the connection can't be loaded (unknown id, keyring
/// error); every connect failure is a `ConnectionHealth`.
#[tauri::command]
#[specta::specta]
pub async fn check_connection(
    config: State<'_, Config>,
    registry: State<'_, DriverRegistry>,
    id: Uuid,
) -> Result<ConnectionHealth, DbError> {
    let (driver_id, connect_config): (String, ConnectConfig) = blocking(&config, move |c| {
        let repo = c.connections();
        let saved = repo.get(id)?;
        Ok((saved.driver, repo.resolve(id, None)?))
    })
    .await?;
    if let Auth::PasswordPrompt { .. } = connect_config.auth {
        return Ok(ConnectionHealth {
            state: HealthState::NeedsPassword,
            message: None,
            server_version: None,
        });
    }
    let driver = match driver(&registry, &driver_id) {
        Ok(driver) => driver,
        Err(err) => return Ok(ConnectionHealth::from_error(&err)),
    };
    match driver.connect(&connect_config).await {
        Ok(conn) => {
            let version = conn.server_version().clone();
            conn.close().await;
            Ok(ConnectionHealth {
                state: HealthState::Online,
                message: None,
                server_version: Some(version),
            })
        }
        Err(err) => Ok(ConnectionHealth::from_error(&err)),
    }
}

fn password_required() -> DbError {
    DbError::new(ErrorKind::Auth, "A password is required to connect").with_code(PASSWORD_REQUIRED)
}

/// Opens a saved connection and keeps it as a session.
///
/// `password` is used once and never saved. Without it, a connection that
/// has no saved password fails with code `PASSWORD_REQUIRED`.
#[tauri::command]
#[specta::specta]
pub async fn connect(
    config: State<'_, Config>,
    registry: State<'_, DriverRegistry>,
    sessions: State<'_, SessionManager>,
    id: Uuid,
    password: Option<String>,
) -> Result<SessionInfo, DbError> {
    let (driver_id, connect_config): (String, ConnectConfig) = blocking(&config, move |c| {
        let repo = c.connections();
        let saved = repo.get(id)?;
        Ok((saved.driver, repo.resolve(id, password.map(Secret::new))?))
    })
    .await?;
    if let Auth::PasswordPrompt { .. } = connect_config.auth {
        return Err(password_required());
    }
    let driver = driver(&registry, &driver_id)?;

    let conn = match driver.connect(&connect_config).await {
        Ok(conn) => conn,
        Err(err) => {
            record(&config, id, failed(&err), false).await;
            return Err(err);
        }
    };
    let version = short_version(conn.server_version());
    let session = sessions
        .insert(Session {
            id: Uuid::new_v4(),
            connection_id: id,
            driver_id: driver.id(),
            conn,
        })
        .await;
    record(&config, id, succeeded(version), true).await;
    Ok(session.info())
}

fn succeeded(server_version: String) -> ConnectionStatus {
    ConnectionStatus {
        at: Utc::now(),
        ok: true,
        error_kind: None,
        message: None,
        server_version: Some(server_version),
    }
}

fn failed(err: &DbError) -> ConnectionStatus {
    let kind = serde_json::to_value(err.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned));
    ConnectionStatus {
        at: Utc::now(),
        ok: false,
        error_kind: kind,
        message: Some(err.message.clone()),
        server_version: None,
    }
}

/// Saves the outcome of a connect. Bookkeeping only: failures are logged,
/// never returned, so they can't hide the connect result.
async fn record(config: &Config, id: Uuid, status: ConnectionStatus, touch: bool) {
    let result = blocking(config, move |c| {
        let repo = c.connections();
        if touch {
            repo.touch(id)?;
        }
        repo.set_status(id, &status)
    })
    .await;
    if let Err(err) = result {
        tracing::warn!(%id, error = %err, "failed to record connection status");
    }
}

#[tauri::command]
#[specta::specta]
pub async fn disconnect(
    sessions: State<'_, SessionManager>,
    session_id: Uuid,
) -> Result<(), DbError> {
    if sessions.close(session_id).await {
        Ok(())
    } else {
        Err(DbError::new(
            ErrorKind::NotFound,
            format!("session {session_id} not found"),
        ))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn list_sessions(sessions: State<'_, SessionManager>) -> Result<Vec<SessionInfo>, DbError> {
    Ok(sessions.list().await.iter().map(|s| s.info()).collect())
}

/// Runs the commands against a real server:
/// SWIFTQL_PG_URL=postgres://user:pass@host/db cargo test --manifest-path src-tauri/Cargo.toml -- --ignored
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use config_store::{ConfigManager, MemoryBackend};
    use driver_core::connect::Target;
    use serde_json::json;
    use tauri::{Manager, test::MockRuntime};

    use super::*;

    struct Pg {
        host: String,
        port: u16,
        user: String,
        password: String,
        database: String,
    }

    fn pg() -> Pg {
        let url = std::env::var("SWIFTQL_PG_URL").expect("set SWIFTQL_PG_URL");
        let rest = url.split_once("://").unwrap().1;
        let (creds, rest) = rest.split_once('@').unwrap();
        let (user, password) = creds.split_once(':').unwrap();
        let (hostport, database) = rest.split_once('/').unwrap();
        let (host, port) = hostport.split_once(':').unwrap_or((hostport, "5432"));
        Pg {
            host: host.into(),
            port: port.parse().unwrap(),
            user: user.into(),
            password: password.into(),
            database: database.into(),
        }
    }

    fn input(pg: &Pg, password: &str, save: bool) -> ConnectionInput {
        serde_json::from_value(json!({
            "name": "e2e",
            "driver": "postgres",
            "target": { "method": "host", "host": pg.host, "port": pg.port, "database": pg.database },
            "auth": { "kind": "password", "user": pg.user, "password": password },
            "tls": { "mode": "disable" },
            "savePassword": save,
        }))
        .unwrap()
    }

    fn app(dir: &std::path::Path) -> tauri::App<MockRuntime> {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let config: Config =
            Arc::new(ConfigManager::open_with(dir, Arc::new(MemoryBackend::new())).unwrap());
        app.manage(config);
        app.manage(DriverRegistry::new());
        app.manage(SessionManager::default());
        app
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs SWIFTQL_PG_URL"]
    async fn test_save_connect_disconnect() {
        let pg = pg();
        let dir = tempfile::tempdir().unwrap();
        let app = app(dir.path());

        // Test: wrong password is an auth error, right one gives the version.
        let err = test_connection(app.state(), app.state(), input(&pg, "wrong", false))
            .await
            .unwrap_err();
        assert_eq!(err.kind, ErrorKind::Auth, "{err:?}");
        let version = test_connection(app.state(), app.state(), input(&pg, &pg.password, false))
            .await
            .unwrap();
        assert_eq!(version.product, "PostgreSQL");

        // Saved without a password: connect asks for one.
        let config = app.state::<Config>();
        let saved = config.connections().save(input(&pg, &pg.password, false)).unwrap();
        let err = connect(app.state(), app.state(), app.state(), saved.id, None)
            .await
            .unwrap_err();
        assert_eq!(err.code.as_deref(), Some(PASSWORD_REQUIRED));

        // A wrong typed password records a failed status.
        let err = connect(app.state(), app.state(), app.state(), saved.id, Some("wrong".into()))
            .await
            .unwrap_err();
        assert_eq!(err.kind, ErrorKind::Auth);
        let status = config.connections().get(saved.id).unwrap().last_status.unwrap();
        assert!(!status.ok);
        assert_eq!(status.error_kind.as_deref(), Some("auth"));

        // The right one opens a session and records success.
        let session = connect(app.state(), app.state(), app.state(), saved.id, Some(pg.password.clone()))
            .await
            .unwrap();
        assert_eq!(session.connection_id, saved.id);
        assert_eq!(session.driver_id, "postgres");
        let after = config.connections().get(saved.id).unwrap();
        assert!(after.last_used_at.is_some());
        let status = after.last_status.unwrap();
        assert!(status.ok);
        assert_eq!(status.server_version, Some(short_version(&session.server_version)));
        assert_eq!(list_sessions(app.state()).await.unwrap().len(), 1);

        disconnect(app.state(), session.session_id).await.unwrap();
        assert!(list_sessions(app.state()).await.unwrap().is_empty());
        let err = disconnect(app.state(), session.session_id).await.unwrap_err();
        assert_eq!(err.kind, ErrorKind::NotFound);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs SWIFTQL_PG_URL"]
    async fn check_connection_states() {
        let pg = pg();
        let dir = tempfile::tempdir().unwrap();
        let app = app(dir.path());
        let config = app.state::<Config>();
        let save = |input: ConnectionInput| config.connections().save(input).unwrap().id;
        let check = |id| check_connection(app.state(), app.state(), id);

        let good = save(input(&pg, &pg.password, true));
        let health = check(good).await.unwrap();
        assert_eq!(health.state, HealthState::Online, "{health:?}");
        assert_eq!(health.server_version.unwrap().product, "PostgreSQL");

        let wrong_password = save(input(&pg, "wrong", true));
        let health = check(wrong_password).await.unwrap();
        assert_eq!(health.state, HealthState::Misconfigured, "{health:?}");
        assert!(health.message.is_some());

        let mut missing_db = input(&pg, &pg.password, true);
        missing_db.target = Target::Host {
            host: pg.host.clone(),
            port: Some(pg.port),
            database: Some("swiftql_no_such_db".into()),
        };
        let health = check(save(missing_db)).await.unwrap();
        assert_eq!(health.state, HealthState::Misconfigured, "{health:?}");

        let mut unreachable = input(&pg, &pg.password, true);
        unreachable.target = Target::Host {
            host: "127.0.0.1".into(),
            port: Some(1),
            database: Some(pg.database.clone()),
        };
        let health = check(save(unreachable)).await.unwrap();
        assert_eq!(health.state, HealthState::Offline, "{health:?}");

        let no_password = save(input(&pg, &pg.password, false));
        let health = check(no_password).await.unwrap();
        assert_eq!(health.state, HealthState::NeedsPassword, "{health:?}");

        // Live only: nothing is recorded.
        let saved = config.connections().get(good).unwrap();
        assert!(saved.last_status.is_none());
        assert!(saved.last_used_at.is_none());
    }
}
