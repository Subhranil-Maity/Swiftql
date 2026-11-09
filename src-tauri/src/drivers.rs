//! Every database driver the app ships with, keyed by `Driver::id`.

use std::{collections::HashMap, sync::Arc};

use driver_core::{
    capabilities::Capabilities,
    connect::{AuthKind, FieldSpec, TargetKind},
    driver::Driver,
    error::DbError,
};
use driver_postgres::PgDriver;
use driver_sqlite::SqliteDriver;
use serde::Serialize;
use specta::Type;
use tauri::State;

pub struct DriverRegistry(HashMap<&'static str, Arc<dyn Driver>>);

impl DriverRegistry {
    pub fn new() -> Self {
        let drivers: [Arc<dyn Driver>; 2] = [Arc::new(PgDriver), Arc::new(SqliteDriver)];
        Self(drivers.into_iter().map(|d| (d.id(), d)).collect())
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Driver>> {
        self.0.get(id).cloned()
    }

    /// Sorted by name so the UI order is stable.
    pub fn all(&self) -> Vec<Arc<dyn Driver>> {
        let mut drivers: Vec<_> = self.0.values().cloned().collect();
        drivers.sort_by_key(|d| d.name());
        drivers
    }
}

/// What the connection form needs to know about a driver.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DriverInfo {
    pub id: &'static str,
    pub name: &'static str,
    /// `data:` URL, ready for `<img src>`.
    pub image: Option<String>,
    pub capabilities: Capabilities,
    pub supported_targets: Vec<TargetKind>,
    pub supported_auth: Vec<AuthKind>,
    pub extra_options: Vec<FieldSpec>,
}

impl DriverInfo {
    fn of(driver: &dyn Driver) -> Self {
        Self {
            id: driver.id(),
            name: driver.name(),
            // TODO: Driver::image has no mime type; assumes SVG
            image: driver
                .image()
                .map(|b64| format!("data:image/svg+xml;base64,{b64}")),
            capabilities: driver.capabilities(),
            supported_targets: driver.supported_targets().to_vec(),
            supported_auth: driver.supported_auth().to_vec(),
            extra_options: driver.extra_options(),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn list_drivers(registry: State<'_, DriverRegistry>) -> Result<Vec<DriverInfo>, DbError> {
    Ok(registry
        .all()
        .iter()
        .map(|d| DriverInfo::of(d.as_ref()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_is_registered() {
        let registry = DriverRegistry::new();
        let pg = registry.get("postgres").expect("postgres driver");
        let info = DriverInfo::of(pg.as_ref());
        assert_eq!(info.name, "PostgreSQL");
        assert!(
            info.image
                .unwrap()
                .starts_with("data:image/svg+xml;base64,")
        );
    }

    #[test]
    fn sqlite_is_registered() {
        let registry = DriverRegistry::new();
        let sqlite = registry.get("sqlite").expect("sqlite driver");
        let info = DriverInfo::of(sqlite.as_ref());
        assert_eq!(info.name, "SQLite");
        assert_eq!(info.supported_targets, vec![TargetKind::File]);
        assert!(!info.capabilities.listen_notify);
    }
}
