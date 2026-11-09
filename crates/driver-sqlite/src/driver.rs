use std::path::PathBuf;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use driver_core::{
    capabilities::Capabilities,
    connect::{Auth, AuthKind, ConnectConfig, FieldKind, FieldSpec, Target, TargetKind, TlsConfig},
    connection::Connection,
    dbresults::DbResult,
    driver::Driver,
    error::{DbError, ErrorKind},
};

use crate::{capabilities, connection::SqliteConnection};

const IMAGE_BYTES: &[u8] = include_bytes!("../../../assets/sqlite.svg");

/// `ConnectConfig::options` key: open the file read-only.
pub const READ_ONLY: &str = "read_only";

pub struct SqliteDriver;

#[async_trait]
impl Driver for SqliteDriver {
    fn name(&self) -> &'static str {
        "SQLite"
    }

    fn id(&self) -> &'static str {
        "sqlite"
    }

    fn image(&self) -> Option<String> {
        Some(STANDARD.encode(IMAGE_BYTES))
    }

    fn capabilities(&self) -> Capabilities {
        capabilities::static_capabilities()
    }

    fn supported_targets(&self) -> &'static [TargetKind] {
        &[TargetKind::File]
    }

    fn supported_auth(&self) -> &'static [AuthKind] {
        &[AuthKind::None]
    }

    fn extra_options(&self) -> Vec<FieldSpec> {
        vec![FieldSpec {
            key: READ_ONLY,
            label: "Open read-only",
            kind: FieldKind::Bool,
            required: false,
            default: Some("false".into()),
        }]
    }

    /// `sqlite:///path/to.db`, `sqlite:path/to.db` or `file:path/to.db`.
    fn parse_url(&self, url: &str) -> DbResult<ConnectConfig> {
        Ok(ConnectConfig {
            target: Target::File {
                path: path_from_url(url)?,
            },
            auth: Auth::None,
            tls: TlsConfig::default(),
            tunnel: None,
            options: Default::default(),
            timeouts: Default::default(),
            host_keys: None,
        })
    }

    async fn test_conn(&self, config: &ConnectConfig) -> DbResult<()> {
        let conn = self.connect(config).await?;
        let result = conn.ping().await;
        conn.close().await;
        result
    }

    async fn connect(&self, config: &ConnectConfig) -> DbResult<Box<dyn Connection>> {
        if config.tunnel.is_some() {
            return Err(unsupported(
                "SQLite files can't be opened through an SSH tunnel",
            ));
        }
        let path = match &config.target {
            Target::File { path } => path.clone(),
            Target::Url { url } => path_from_url(url)?,
            _ => return Err(unsupported("SQLite opens a database file; pick a file")),
        };
        let read_only = config.options.get(READ_ONLY).is_some_and(|v| v == "true");
        Ok(Box::new(SqliteConnection::open(path, read_only).await?))
    }
}

fn unsupported(message: &str) -> DbError {
    DbError::new(ErrorKind::Unsupported, message)
}

fn path_from_url(url: &str) -> DbResult<PathBuf> {
    let path = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))
        .or_else(|| url.strip_prefix("file:"))
        .unwrap_or(url);
    // sqlite:///C:/x.db → C:/x.db on Windows; /home/x.db elsewhere.
    let path = if cfg!(windows) && path.starts_with('/') && path.as_bytes().get(2) == Some(&b':') {
        &path[1..]
    } else {
        path
    };
    let path = path.split('?').next().unwrap_or_default();
    if path.is_empty() {
        return Err(DbError::new(ErrorKind::Syntax, "the URL has no file path"));
    }
    Ok(PathBuf::from(path))
}
