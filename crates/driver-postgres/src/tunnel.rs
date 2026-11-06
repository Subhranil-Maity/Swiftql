//! SSH tunnels: `ConnectConfig::tunnel` → an `ssh_tunnel::Tunnel`, and
//! tunnel errors → `DbError`s with codes the app can act on.

use std::{sync::Arc, time::Duration};

use driver_core::{
    connect::{ConnectConfig, HostKeyPolicy, SshAuth, SshSource, SshTunnel, ssh_codes},
    dbresults::DbResult,
    error::{DbError, ErrorKind},
};
use ssh_tunnel::{Auth, Endpoint, Error, HostKeys, Options, SshConfig, Tunnel};

/// Where a connection's bytes go: straight to the server, or through SSH.
#[derive(Clone)]
pub enum Route {
    Direct,
    Tunnel {
        tunnel: Arc<Tunnel>,
        /// The database host and port as the SSH server sees them.
        host: String,
        port: u16,
    },
}

pub async fn open(tunnel: &SshTunnel, config: &ConnectConfig) -> DbResult<Arc<Tunnel>> {
    let auth = match &tunnel.auth {
        SshAuth::Password { password } => Auth::Password(password.expose().to_owned()),
        SshAuth::PasswordPrompt => Auth::PasswordPrompt,
        SshAuth::Key { path, passphrase } => Auth::Key {
            path: ssh_tunnel::config::expand_tilde(&path.to_string_lossy()),
            passphrase: passphrase.as_ref().map(|p| p.expose().to_owned()),
        },
        SshAuth::Agent => Auth::Agent,
        SshAuth::Auto => Auth::Auto,
    };
    let endpoint = match &tunnel.source {
        SshSource::Manual => Endpoint {
            host: tunnel.host.clone(),
            port: tunnel.port.unwrap_or(22),
            user: tunnel.user.clone(),
            auth,
            identity_files: Vec::new(),
            jumps: Vec::new(),
        },
        SshSource::Config { alias } => {
            let ssh_config = SshConfig::load_default().map_err(|err| {
                DbError::new(
                    ErrorKind::Internal,
                    format!("couldn't read ~/.ssh/config: {err}"),
                )
            })?;
            Endpoint::from_config(
                &ssh_config,
                alias,
                &tunnel.host,
                tunnel.port,
                &tunnel.user,
                auth,
            )
            .map_err(ssh_error)?
        }
    };
    if endpoint.host.is_empty() {
        return Err(DbError::new(
            ErrorKind::Syntax,
            "the SSH tunnel has no host",
        ));
    }

    let keys = match &config.host_keys {
        Some(check) => HostKeys {
            policy: match check.policy {
                HostKeyPolicy::Ask => ssh_tunnel::HostKeyPolicy::Ask,
                HostKeyPolicy::AcceptNew => ssh_tunnel::HostKeyPolicy::AcceptNew,
                HostKeyPolicy::Strict => ssh_tunnel::HostKeyPolicy::Strict,
            },
            user_file: check.user_known_hosts.clone(),
            app_file: check.app_known_hosts.clone(),
        },
        // Used without the app (tests): only hosts the user's ssh already knows.
        None => HostKeys {
            policy: ssh_tunnel::HostKeyPolicy::Strict,
            user_file: ssh_tunnel::config::default_known_hosts_path(),
            app_file: std::env::temp_dir().join("swiftql_known_hosts"),
        },
    };
    let options = Options {
        connect_timeout: Duration::from_secs(u64::from(
            config
                .timeouts
                .connect_secs
                .filter(|&s| s > 0)
                .unwrap_or(10),
        )),
        keepalive: config
            .timeouts
            .keepalive_secs
            .filter(|&s| s > 0)
            .map(|s| Duration::from_secs(u64::from(s))),
    };
    Tunnel::connect(&endpoint, &keys, &options)
        .await
        .map(Arc::new)
        .map_err(ssh_error)
}

pub fn ssh_error(err: Error) -> DbError {
    let message = err.to_string();
    match err {
        Error::HostUnknown(info) => DbError::new(ErrorKind::Auth, message)
            .with_code(ssh_codes::HOST_UNKNOWN)
            .with_detail(serde_json::to_string(&info).unwrap_or_default()),
        Error::HostChanged { .. } => {
            DbError::new(ErrorKind::Auth, message).with_code(ssh_codes::HOST_CHANGED)
        }
        Error::PasswordRequired => {
            DbError::new(ErrorKind::Auth, message).with_code(ssh_codes::PASSWORD_REQUIRED)
        }
        Error::PassphraseRequired(_) => {
            DbError::new(ErrorKind::Auth, message).with_code(ssh_codes::PASSPHRASE_REQUIRED)
        }
        Error::AuthFailed { .. } => DbError::new(ErrorKind::Auth, message),
        Error::Connect(_) | Error::Ssh(_) => DbError::new(ErrorKind::Network, message),
        Error::Timeout(_) => DbError::new(ErrorKind::Timeout, message),
        Error::Config(_) => DbError::new(ErrorKind::Syntax, message),
    }
}
