//! Host key checking against the user's `~/.ssh/known_hosts` (read only)
//! and SwiftQL's own known_hosts file (where hosts the user trusts in the app
//! are written).

use std::path::{Path, PathBuf};

use russh::keys::{
    HashAlg, PublicKey,
    known_hosts::{check_known_hosts_path, learn_known_hosts_path},
};

use crate::{Error, UnknownHost};

/// What to do with a host whose key is in neither file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HostKeyPolicy {
    /// Fail with `Error::HostUnknown` so the user can check the fingerprint.
    #[default]
    Ask,
    /// Trust it and add it to the app's file.
    AcceptNew,
    /// Refuse it.
    Strict,
}

#[derive(Debug, Clone)]
pub struct HostKeys {
    pub policy: HostKeyPolicy,
    pub user_file: Option<PathBuf>,
    pub app_file: PathBuf,
}

impl HostKeys {
    /// Ok when the key is known (or newly trusted by policy).
    pub fn check(&self, host: &str, port: u16, key: &PublicKey) -> Result<(), Error> {
        let files = self
            .user_file
            .iter()
            .map(PathBuf::as_path)
            .chain([self.app_file.as_path()]);
        for file in files {
            match check_known_hosts_path(host, port, key, file) {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(russh::keys::Error::KeyChanged { .. }) => {
                    return Err(Error::HostChanged {
                        host: host.to_owned(),
                        port,
                        fingerprint: fingerprint(key),
                    });
                }
                // An unreadable line or file counts as "not known here".
                Err(err) => {
                    tracing::debug!(file = %file.display(), error = %err, "known_hosts check")
                }
            }
        }
        match self.policy {
            HostKeyPolicy::AcceptNew => {
                learn(&self.app_file, host, port, key)?;
                tracing::info!(%host, port, fingerprint = %fingerprint(key), "trusted new SSH host key");
                Ok(())
            }
            HostKeyPolicy::Ask | HostKeyPolicy::Strict => {
                Err(Error::HostUnknown(Box::new(UnknownHost {
                    host: host.to_owned(),
                    port,
                    key_type: key.algorithm().as_str().to_owned(),
                    fingerprint: fingerprint(key),
                    key: key.to_openssh().unwrap_or_default(),
                })))
            }
        }
    }
}

fn fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

fn learn(file: &Path, host: &str, port: u16, key: &PublicKey) -> Result<(), Error> {
    learn_known_hosts_path(host, port, key, file)
        .map_err(|err| Error::Config(format!("couldn't write {}: {err}", file.display())))
}

/// Adds a host the user confirmed (an `UnknownHost::key` line) to the app's file.
pub fn trust_host(app_file: &Path, host: &str, port: u16, openssh_key: &str) -> Result<(), Error> {
    let key = PublicKey::from_openssh(openssh_key)
        .map_err(|err| Error::Config(format!("invalid host key: {err}")))?;
    learn(app_file, host, port, &key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJdD7y3aLq454yWBdwLWbieU1ebz9/cu7/QEXn9OIeZJ";
    const OTHER: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIA6rWI3G1sz07Qn1L2Z5EvVYa/C0CgKJjU8JJy0+R2sZ";

    fn keys(dir: &Path, policy: HostKeyPolicy) -> HostKeys {
        HostKeys {
            policy,
            user_file: Some(dir.join("user_known_hosts")),
            app_file: dir.join("app_known_hosts"),
        }
    }

    #[test]
    fn unknown_then_trusted_then_changed() {
        let dir = tempfile::tempdir().unwrap();
        let k = keys(dir.path(), HostKeyPolicy::Ask);
        let key = PublicKey::from_openssh(KEY).unwrap();
        let unknown = k.check("db.example", 2222, &key).unwrap_err();
        let Error::HostUnknown(info) = unknown else {
            panic!("{unknown:?}")
        };
        assert!(info.fingerprint.starts_with("SHA256:"));

        trust_host(&k.app_file, "db.example", 2222, &info.key).unwrap();
        k.check("db.example", 2222, &key).unwrap();

        let other = PublicKey::from_openssh(OTHER).unwrap();
        assert!(matches!(
            k.check("db.example", 2222, &other),
            Err(Error::HostChanged { .. })
        ));
    }

    #[test]
    fn accept_new_learns() {
        let dir = tempfile::tempdir().unwrap();
        let k = keys(dir.path(), HostKeyPolicy::AcceptNew);
        let key = PublicKey::from_openssh(KEY).unwrap();
        k.check("h", 22, &key).unwrap();
        let strict = HostKeys {
            policy: HostKeyPolicy::Strict,
            ..k
        };
        strict.check("h", 22, &key).unwrap();
    }
}
