//! SSH tunnels for database drivers. A `Tunnel` is an authenticated SSH
//! session (through any ProxyJump hops); `dial` opens a `direct-tcpip`
//! channel to the database through it. No local port is opened, so other
//! processes on the machine can't use the forward.

pub mod config;
mod known_hosts;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use russh::{
    ChannelStream,
    client::{self, AuthResult, Handle, Msg},
    keys::{
        HashAlg, PrivateKeyWithHashAlg, PublicKey, PublicKeyOrCertificate,
        agent::{AgentIdentity, client::AgentClient},
    },
};
use serde::Serialize;

pub use config::{HostConfig, SshConfig};

/// A forwarded connection: `AsyncRead + AsyncWrite`.
pub type TunnelStream = ChannelStream<Msg>;
pub use known_hosts::{HostKeyPolicy, HostKeys, trust_host};

/// Default keys tried by `Auth::Auto`, as `ssh` does.
const DEFAULT_KEYS: &[&str] = &["id_ed25519", "id_ecdsa", "id_rsa"];

/// ProxyJump chains longer than this are refused (loops).
const MAX_HOPS: usize = 8;

#[derive(Debug, Clone)]
pub enum Auth {
    Password(String),
    /// The password isn't known; connecting fails with `PasswordRequired`.
    PasswordPrompt,
    Key {
        path: PathBuf,
        passphrase: Option<String>,
    },
    Agent,
    /// The endpoint's identity files, the default keys, then the agent.
    Auto,
}

/// One SSH server to log in to.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: Auth,
    /// IdentityFile entries from the SSH config, for `Auth::Auto`.
    pub identity_files: Vec<PathBuf>,
    /// Hops to go through first, outermost first (ProxyJump).
    pub jumps: Vec<Endpoint>,
}

impl Endpoint {
    /// An endpoint for `alias` from the SSH config, with its ProxyJump hops.
    /// Empty `host` / `user` and a None `port` take the config's values.
    pub fn from_config(
        config: &SshConfig,
        alias: &str,
        host: &str,
        port: Option<u16>,
        user: &str,
        auth: Auth,
    ) -> Result<Self, Error> {
        let mut endpoint = Self::from_host_config(config, &config.resolve(alias), 0)?;
        if !host.is_empty() {
            endpoint.host = host.to_owned();
        }
        if let Some(port) = port {
            endpoint.port = port;
        }
        if !user.is_empty() {
            endpoint.user = user.to_owned();
        }
        endpoint.auth = auth;
        Ok(endpoint)
    }

    fn from_host_config(
        config: &SshConfig,
        host: &HostConfig,
        depth: usize,
    ) -> Result<Self, Error> {
        if depth > MAX_HOPS {
            return Err(Error::Config(format!(
                "ProxyJump of {} goes deeper than {MAX_HOPS} hops",
                host.alias
            )));
        }
        let mut jumps = Vec::new();
        for hop in &host.proxy_jump {
            let (user, name, port) = config::parse_hop(hop);
            let mut hop_config = config.resolve(&name);
            if user.is_some() {
                hop_config.user = user;
            }
            if port.is_some() {
                hop_config.port = port;
            }
            let mut hop_endpoint = Self::from_host_config(config, &hop_config, depth + 1)?;
            // A hop's own hops come before it.
            jumps.append(&mut hop_endpoint.jumps);
            jumps.push(hop_endpoint);
        }
        Ok(Self {
            host: host.host().to_owned(),
            port: host.port.unwrap_or(22),
            user: host.user.clone().unwrap_or_else(local_user),
            auth: Auth::Auto,
            identity_files: host.identity_files.clone(),
            jumps,
        })
    }
}

/// The OS user name, the default SSH user.
pub fn local_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "root".into())
}

#[derive(Debug, Clone)]
pub struct Options {
    pub connect_timeout: Duration,
    /// SSH keepalive; None = off.
    pub keepalive: Option<Duration>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            keepalive: Some(Duration::from_secs(60)),
        }
    }
}

/// A host key that isn't known yet, for the user to check.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownHost {
    pub host: String,
    pub port: u16,
    pub key_type: String,
    /// "SHA256:..." as `ssh` prints it.
    pub fingerprint: String,
    /// The OpenSSH public key line, to trust it with `trust_host`.
    pub key: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("The SSH host key of {} isn't known (fingerprint {})", .0.host, .0.fingerprint)]
    HostUnknown(Box<UnknownHost>),
    #[error(
        "The SSH host key of {host}:{port} has changed (now {fingerprint}). This can mean someone is intercepting the connection; if the server was reinstalled, remove its old key from known_hosts."
    )]
    HostChanged {
        host: String,
        port: u16,
        fingerprint: String,
    },
    #[error("An SSH password is required")]
    PasswordRequired,
    #[error("The SSH key {} is encrypted; a passphrase is required", .0.display())]
    PassphraseRequired(PathBuf),
    #[error("SSH authentication as {user}@{host} failed: {reason}")]
    AuthFailed {
        user: String,
        host: String,
        reason: String,
    },
    #[error("Couldn't reach the SSH server {0}")]
    Connect(String),
    #[error("Timed out connecting to the SSH server {0}")]
    Timeout(String),
    #[error("SSH config: {0}")]
    Config(String),
    #[error("SSH: {0}")]
    Ssh(String),
}

impl From<russh::Error> for Error {
    fn from(err: russh::Error) -> Self {
        Error::Ssh(err.to_string())
    }
}

/// Checks the server's host key; a rejection's reason is kept in `verdict`.
struct Client {
    host: String,
    port: u16,
    keys: HostKeys,
    verdict: Arc<Mutex<Option<Error>>>,
}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let key = match key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            PublicKeyOrCertificate::Certificate(cert) => PublicKey::from(cert.public_key().clone()),
        };
        match self.keys.check(&self.host, self.port, &key) {
            Ok(()) => Ok(true),
            Err(err) => {
                *self.verdict.lock().unwrap_or_else(|e| e.into_inner()) = Some(err);
                Ok(false)
            }
        }
    }
}

/// A logged-in SSH session; dropping it closes the session and its hops.
pub struct Tunnel {
    handle: Handle<Client>,
    /// Sessions to the ProxyJump hops, kept alive for `handle`.
    _hops: Vec<Handle<Client>>,
    target: String,
}

impl Tunnel {
    /// Connects and logs in, through the endpoint's ProxyJump hops.
    pub async fn connect(
        endpoint: &Endpoint,
        keys: &HostKeys,
        options: &Options,
    ) -> Result<Self, Error> {
        if matches!(endpoint.auth, Auth::PasswordPrompt) {
            return Err(Error::PasswordRequired);
        }
        let target = format!("{}:{}", endpoint.host, endpoint.port);
        let config = Arc::new(client::Config {
            keepalive_interval: options.keepalive,
            keepalive_max: 3,
            ..client::Config::default()
        });

        let mut hops: Vec<Handle<Client>> = Vec::new();
        let chain: Vec<&Endpoint> = endpoint
            .jumps
            .iter()
            .chain(std::iter::once(endpoint))
            .collect();
        for hop in chain {
            let label = format!("{}:{}", hop.host, hop.port);
            let attempt = async {
                let handle = match hops.last() {
                    None => open_tcp(&config, hop, keys).await?,
                    Some(previous) => open_through(previous, &config, hop, keys).await?,
                };
                Ok::<_, Error>(handle)
            };
            let mut handle = tokio::time::timeout(options.connect_timeout, attempt)
                .await
                .map_err(|_| Error::Timeout(label.clone()))??;
            tokio::time::timeout(options.connect_timeout, authenticate(&mut handle, hop))
                .await
                .map_err(|_| Error::Timeout(label))??;
            hops.push(handle);
        }
        let handle = hops.pop().expect("the chain ends with the endpoint");
        tracing::debug!(%target, hops = hops.len(), "SSH tunnel up");
        Ok(Self {
            handle,
            _hops: hops,
            target,
        })
    }

    /// A stream to `host:port` as seen from the SSH server.
    pub async fn dial(&self, host: &str, port: u16) -> Result<ChannelStream<Msg>, Error> {
        let channel = self
            .handle
            .channel_open_direct_tcpip(host, u32::from(port), "127.0.0.1", 0)
            .await
            .map_err(|err| {
                Error::Ssh(format!(
                    "forwarding to {host}:{port} via {} failed: {err}",
                    self.target
                ))
            })?;
        Ok(channel.into_stream())
    }

    pub fn is_closed(&self) -> bool {
        self.handle.is_closed()
    }
}

fn client_for(hop: &Endpoint, keys: &HostKeys) -> (Client, Arc<Mutex<Option<Error>>>) {
    let verdict = Arc::new(Mutex::new(None));
    let client = Client {
        host: hop.host.clone(),
        port: hop.port,
        keys: keys.clone(),
        verdict: Arc::clone(&verdict),
    };
    (client, verdict)
}

/// A failed handshake reports why the host key was refused, if it was.
fn handshake_error(err: russh::Error, verdict: &Mutex<Option<Error>>, label: &str) -> Error {
    if let Some(reason) = verdict.lock().unwrap_or_else(|e| e.into_inner()).take() {
        return reason;
    }
    match err {
        russh::Error::IO(io) => Error::Connect(format!("{label}: {io}")),
        other => Error::Ssh(format!("{label}: {other}")),
    }
}

async fn open_tcp(
    config: &Arc<client::Config>,
    hop: &Endpoint,
    keys: &HostKeys,
) -> Result<Handle<Client>, Error> {
    let (client, verdict) = client_for(hop, keys);
    let label = format!("{}:{}", hop.host, hop.port);
    client::connect(Arc::clone(config), (hop.host.as_str(), hop.port), client)
        .await
        .map_err(|err| handshake_error(err, &verdict, &label))
}

async fn open_through(
    previous: &Handle<Client>,
    config: &Arc<client::Config>,
    hop: &Endpoint,
    keys: &HostKeys,
) -> Result<Handle<Client>, Error> {
    let label = format!("{}:{}", hop.host, hop.port);
    let channel = previous
        .channel_open_direct_tcpip(hop.host.as_str(), u32::from(hop.port), "127.0.0.1", 0)
        .await
        .map_err(|err| Error::Connect(format!("{label} (via ProxyJump): {err}")))?;
    let (client, verdict) = client_for(hop, keys);
    client::connect_stream(Arc::clone(config), channel.into_stream(), client)
        .await
        .map_err(|err| handshake_error(err, &verdict, &label))
}

async fn authenticate(handle: &mut Handle<Client>, hop: &Endpoint) -> Result<(), Error> {
    let failed = |reason: String| Error::AuthFailed {
        user: hop.user.clone(),
        host: hop.host.clone(),
        reason,
    };
    match &hop.auth {
        Auth::PasswordPrompt => Err(Error::PasswordRequired),
        Auth::Password(password) => {
            let result = handle.authenticate_password(&hop.user, password).await?;
            success(&result)
                .then_some(())
                .ok_or_else(|| failed("wrong password".into()))
        }
        Auth::Key { path, passphrase } => {
            let key = load_key(path, passphrase.as_deref())?;
            if try_key(handle, &hop.user, key).await? {
                Ok(())
            } else {
                Err(failed(format!(
                    "the server didn't accept {}",
                    path.display()
                )))
            }
        }
        Auth::Agent => {
            if try_agent(handle, &hop.user).await? {
                Ok(())
            } else {
                Err(failed("the SSH agent has no key the server accepts".into()))
            }
        }
        Auth::Auto => {
            let mut encrypted = Vec::new();
            for path in candidate_keys(&hop.identity_files) {
                match load_key(&path, None) {
                    Ok(key) => {
                        if try_key(handle, &hop.user, key).await? {
                            return Ok(());
                        }
                    }
                    Err(Error::PassphraseRequired(path)) => encrypted.push(path),
                    Err(err) => {
                        tracing::debug!(path = %path.display(), error = %err, "skipping SSH key")
                    }
                }
            }
            if try_agent(handle, &hop.user).await.unwrap_or(false) {
                return Ok(());
            }
            let mut reason = "no key or agent identity was accepted".to_owned();
            if !encrypted.is_empty() {
                let names: Vec<String> =
                    encrypted.iter().map(|p| p.display().to_string()).collect();
                reason.push_str(&format!(
                    "; encrypted keys were skipped ({}): add them to the SSH agent, or choose Key authentication",
                    names.join(", ")
                ));
            }
            Err(failed(reason))
        }
    }
}

fn success(result: &AuthResult) -> bool {
    matches!(result, AuthResult::Success)
}

/// Identity files from the config, else the default keys, that exist.
fn candidate_keys(identity_files: &[PathBuf]) -> Vec<PathBuf> {
    let list: Vec<PathBuf> = if identity_files.is_empty() {
        let dir = config::home_dir()
            .map(|h| h.join(".ssh"))
            .unwrap_or_default();
        DEFAULT_KEYS.iter().map(|k| dir.join(k)).collect()
    } else {
        identity_files.to_vec()
    };
    list.into_iter().filter(|p| p.is_file()).collect()
}

fn load_key(path: &Path, passphrase: Option<&str>) -> Result<russh::keys::PrivateKey, Error> {
    match russh::keys::load_secret_key(path, passphrase) {
        Ok(key) => Ok(key),
        Err(russh::keys::Error::KeyIsEncrypted) => {
            Err(Error::PassphraseRequired(path.to_path_buf()))
        }
        Err(err) if passphrase.is_some() && is_decrypt_error(&err) => Err(Error::AuthFailed {
            user: String::new(),
            host: String::new(),
            reason: format!("wrong passphrase for {}", path.display()),
        }),
        Err(err) => Err(Error::Config(format!(
            "couldn't read the key {}: {err}",
            path.display()
        ))),
    }
}

fn is_decrypt_error(err: &russh::keys::Error) -> bool {
    let text = err.to_string().to_ascii_lowercase();
    text.contains("decrypt") || text.contains("crypto") || text.contains("passphrase")
}

async fn rsa_hash(handle: &Handle<Client>) -> Option<HashAlg> {
    handle
        .best_supported_rsa_hash()
        .await
        .ok()
        .flatten()
        .flatten()
}

async fn try_key(
    handle: &mut Handle<Client>,
    user: &str,
    key: russh::keys::PrivateKey,
) -> Result<bool, Error> {
    let hash = rsa_hash(handle).await;
    let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
    Ok(success(&handle.authenticate_publickey(user, key).await?))
}

/// Tries every identity the agent holds. False when there is no agent or
/// none was accepted.
async fn try_agent(handle: &mut Handle<Client>, user: &str) -> Result<bool, Error> {
    #[cfg(windows)]
    {
        if let Ok(agent) = AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await
            && agent_login(handle, user, agent).await?
        {
            return Ok(true);
        }
        if let Ok(agent) = AgentClient::connect_pageant().await {
            return agent_login(handle, user, agent).await;
        }
        Ok(false)
    }
    #[cfg(unix)]
    {
        match AgentClient::connect_env().await {
            Ok(agent) => agent_login(handle, user, agent).await,
            Err(_) => Ok(false),
        }
    }
}

async fn agent_login<S>(
    handle: &mut Handle<Client>,
    user: &str,
    mut agent: AgentClient<S>,
) -> Result<bool, Error>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let identities = agent
        .request_identities()
        .await
        .map_err(|err| Error::Ssh(format!("SSH agent: {err}")))?;
    let hash = rsa_hash(handle).await;
    for identity in identities {
        let AgentIdentity::PublicKey { key, .. } = identity else {
            continue; // certificates: not supported here
        };
        match handle
            .authenticate_publickey_with(user, key, hash, &mut agent)
            .await
        {
            Ok(result) if success(&result) => return Ok(true),
            Ok(_) => {}
            Err(err) => tracing::debug!(error = %err, "SSH agent signing failed"),
        }
    }
    Ok(false)
}
