//! SSH helpers for the connection form: hosts from `~/.ssh/config`, and the
//! hosts the user trusted in SwiftQL.

use driver_core::error::{DbError, ErrorKind};
use serde::Serialize;
use specta::Type;
use ssh_tunnel::SshConfig;
use tauri::State;

use crate::{config::Config, connect::app_known_hosts};

/// A `Host` alias from `~/.ssh/config`, resolved as `ssh <alias>` would.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SshConfigHost {
    pub alias: String,
    pub host_name: String,
    pub user: Option<String>,
    pub port: u16,
    pub identity_files: Vec<String>,
    /// ProxyJump hops, outermost first.
    pub proxy_jump: Vec<String>,
}

fn internal(err: impl std::fmt::Display) -> DbError {
    DbError::new(ErrorKind::Internal, err.to_string())
}

/// Hosts named in `~/.ssh/config` (no wildcard patterns); empty without one.
#[tauri::command]
#[specta::specta]
pub async fn list_ssh_hosts() -> Result<Vec<SshConfigHost>, DbError> {
    tauri::async_runtime::spawn_blocking(|| {
        let config = SshConfig::load_default().map_err(internal)?;
        Ok(config
            .aliases()
            .into_iter()
            .map(|alias| {
                let host = config.resolve(&alias);
                SshConfigHost {
                    host_name: host.host().to_owned(),
                    user: host.user.clone(),
                    port: host.port.unwrap_or(22),
                    identity_files: host
                        .identity_files
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect(),
                    proxy_jump: host.proxy_jump.clone(),
                    alias,
                }
            })
            .collect())
    })
    .await
    .map_err(internal)?
}

/// Trusts a host key the user checked (the `key` from an `SSH_HOST_UNKNOWN`
/// error's detail). Written to SwiftQL's own known_hosts file only.
#[tauri::command]
#[specta::specta]
pub async fn trust_ssh_host(
    config: State<'_, Config>,
    host: String,
    port: u16,
    key: String,
) -> Result<(), DbError> {
    let file = app_known_hosts(&config);
    tauri::async_runtime::spawn_blocking(move || {
        ssh_tunnel::trust_host(&file, &host, port, &key)
            .map_err(|err| DbError::new(ErrorKind::Internal, err.to_string()))
    })
    .await
    .map_err(internal)?
}

/// Forgets every host trusted in SwiftQL (`~/.ssh/known_hosts` is untouched).
#[tauri::command]
#[specta::specta]
pub async fn clear_trusted_ssh_hosts(config: State<'_, Config>) -> Result<(), DbError> {
    let file = app_known_hosts(&config);
    match tokio::fs::remove_file(&file).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(internal(err)),
    }
}
