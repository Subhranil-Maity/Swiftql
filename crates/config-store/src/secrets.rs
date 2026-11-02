use std::{collections::HashMap, fmt, sync::Arc, sync::Mutex};

use driver_core::connect::Secret;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ConfigError, ConfigResult};

/// Keyring service name every secret is stored under.
pub const SERVICE: &str = "dev.swiftql";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretKind {
    DbPassword,
    SshPassword,
    SshPassphrase,
}

impl SecretKind {
    pub const ALL: [SecretKind; 3] = [
        SecretKind::DbPassword,
        SecretKind::SshPassword,
        SecretKind::SshPassphrase,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SecretKind::DbPassword => "db_password",
            SecretKind::SshPassword => "ssh_password",
            SecretKind::SshPassphrase => "ssh_passphrase",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretRef {
    pub connection_id: Uuid,
    pub kind: SecretKind,
}

impl SecretRef {
    pub fn new(connection_id: Uuid, kind: SecretKind) -> Self {
        Self {
            connection_id,
            kind,
        }
    }

    /// Keyring account name: `conn:{uuid}:{kind}`.
    pub fn account(&self) -> String {
        format!("conn:{}:{}", self.connection_id, self.kind.as_str())
    }
}

/// Raw secret storage, keyed by account name.
pub trait SecretBackend: Send + Sync {
    fn set(&self, account: &str, value: &str) -> ConfigResult<()>;
    fn get(&self, account: &str) -> ConfigResult<Option<String>>;
    /// Deleting a missing entry is not an error.
    fn delete(&self, account: &str) -> ConfigResult<()>;
}

/// The OS credential store (Credential Manager, Keychain, Secret Service).
#[derive(Debug, Default)]
pub struct KeyringBackend;

impl KeyringBackend {
    fn entry(account: &str) -> ConfigResult<keyring::Entry> {
        Ok(keyring::Entry::new(SERVICE, account)?)
    }

    /// Whether the platform keyring could be initialized.
    pub fn available() -> KeyringStatus {
        match keyring::Entry::store_status() {
            Ok(()) => KeyringStatus {
                available: true,
                message: None,
            },
            Err(err) => KeyringStatus {
                available: false,
                message: Some(err.to_string()),
            },
        }
    }
}

impl SecretBackend for KeyringBackend {
    fn set(&self, account: &str, value: &str) -> ConfigResult<()> {
        Ok(Self::entry(account)?.set_password(value)?)
    }

    fn get(&self, account: &str) -> ConfigResult<Option<String>> {
        match Self::entry(account)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn delete(&self, account: &str) -> ConfigResult<()> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyringStatus {
    pub available: bool,
    pub message: Option<String>,
}

/// In-memory backend for tests.
#[derive(Default)]
pub struct MemoryBackend {
    entries: Mutex<HashMap<String, String>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl SecretBackend for MemoryBackend {
    fn set(&self, account: &str, value: &str) -> ConfigResult<()> {
        self.entries
            .lock()
            .unwrap()
            .insert(account.to_owned(), value.to_owned());
        Ok(())
    }

    fn get(&self, account: &str) -> ConfigResult<Option<String>> {
        Ok(self.entries.lock().unwrap().get(account).cloned())
    }

    fn delete(&self, account: &str) -> ConfigResult<()> {
        self.entries.lock().unwrap().remove(account);
        Ok(())
    }
}

/// Typed wrapper over a backend. Values go in and out as `Secret`, so they
/// never show up in logs.
#[derive(Clone)]
pub struct SecretStore {
    backend: Arc<dyn SecretBackend>,
}

impl SecretStore {
    pub fn new(backend: Arc<dyn SecretBackend>) -> Self {
        Self { backend }
    }

    pub fn set(&self, r: SecretRef, value: &Secret) -> ConfigResult<()> {
        if value.expose().is_empty() {
            return Err(ConfigError::Invalid("cannot save an empty secret".into()));
        }
        self.backend.set(&r.account(), value.expose())
    }

    pub fn get(&self, r: SecretRef) -> ConfigResult<Option<Secret>> {
        Ok(self.backend.get(&r.account())?.map(Secret::new))
    }

    pub fn exists(&self, r: SecretRef) -> ConfigResult<bool> {
        Ok(self.backend.get(&r.account())?.is_some())
    }

    pub fn delete(&self, r: SecretRef) -> ConfigResult<()> {
        self.backend.delete(&r.account())
    }

    /// Removes every secret kind for one connection.
    pub fn delete_all(&self, connection_id: Uuid) -> ConfigResult<()> {
        for kind in SecretKind::ALL {
            self.delete(SecretRef::new(connection_id, kind))?;
        }
        Ok(())
    }
}

impl fmt::Debug for SecretStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretStore(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_name_format() {
        let id = Uuid::nil();
        assert_eq!(
            SecretRef::new(id, SecretKind::DbPassword).account(),
            "conn:00000000-0000-0000-0000-000000000000:db_password"
        );
    }

    #[test]
    fn memory_store_round_trip() {
        let store = SecretStore::new(Arc::new(MemoryBackend::new()));
        let r = SecretRef::new(Uuid::new_v4(), SecretKind::SshPassphrase);
        assert!(store.get(r).unwrap().is_none());
        store.set(r, &Secret::new("hunter2")).unwrap();
        assert_eq!(store.get(r).unwrap().unwrap().expose(), "hunter2");
        store.delete(r).unwrap();
        store.delete(r).unwrap();
        assert!(!store.exists(r).unwrap());
        assert_eq!(format!("{store:?}"), "SecretStore(***)");
    }

    #[test]
    #[ignore = "touches the real OS keyring"]
    fn real_keyring_round_trip() {
        assert!(KeyringBackend::available().available);
        let store = SecretStore::new(Arc::new(KeyringBackend));
        let r = SecretRef::new(Uuid::new_v4(), SecretKind::DbPassword);
        store.set(r, &Secret::new("swiftql-test")).unwrap();
        assert_eq!(store.get(r).unwrap().unwrap().expose(), "swiftql-test");
        store.delete(r).unwrap();
        assert!(store.get(r).unwrap().is_none());
        store.delete(r).unwrap();
    }
}
