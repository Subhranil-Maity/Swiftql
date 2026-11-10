use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::error::ConfigResult;

/// Applied in order; `PRAGMA user_version` holds how many have run.
/// Never edit a migration that has shipped, append a new one instead.
const MIGRATIONS: &[&str] = &[
    // 1: saved connections, their last status, misc app state
    r#"
    CREATE TABLE connections (
        id           TEXT PRIMARY KEY,
        name         TEXT NOT NULL,
        driver       TEXT NOT NULL,
        folder       TEXT,
        color        TEXT,
        sort_order   INTEGER NOT NULL DEFAULT 0,
        config_json  TEXT NOT NULL,
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL,
        last_used_at INTEGER
    );
    CREATE TABLE connection_status (
        connection_id  TEXT PRIMARY KEY REFERENCES connections(id) ON DELETE CASCADE,
        at             INTEGER NOT NULL,
        ok             INTEGER NOT NULL,
        error_kind     TEXT,
        message        TEXT,
        server_version TEXT
    );
    CREATE TABLE kv (
        key        TEXT PRIMARY KEY,
        value_json TEXT NOT NULL
    );
    "#,
];

/// `app.db`. Timestamps are stored as unix milliseconds.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> ConfigResult<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> ConfigResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> ConfigResult<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn kv_get(&self, key: &str) -> ConfigResult<Option<Value>> {
        let json: Option<String> = self
            .lock()
            .query_row("SELECT value_json FROM kv WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(json.map(|j| serde_json::from_str(&j)).transpose()?)
    }

    /// `null` deletes the key.
    pub fn kv_set(&self, key: &str, value: &Value) -> ConfigResult<()> {
        let conn = self.lock();
        if value.is_null() {
            conn.execute("DELETE FROM kv WHERE key = ?1", [key])?;
        } else {
            conn.execute(
                "INSERT INTO kv (key, value_json) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
                params![key, serde_json::to_string(value)?],
            )?;
        }
        Ok(())
    }
}

fn migrate(conn: &mut Connection) -> ConfigResult<()> {
    let applied: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(applied as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
        tracing::info!(version = index + 1, "applied app.db migration");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn user_version(db: &Db) -> i64 {
        db.lock()
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn migrations_are_idempotent_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.db");
        {
            let db = Db::open(&path).unwrap();
            assert_eq!(user_version(&db), MIGRATIONS.len() as i64);
            db.kv_set("k", &json!(1)).unwrap();
        }
        let db = Db::open(&path).unwrap();
        assert_eq!(user_version(&db), MIGRATIONS.len() as i64);
        assert_eq!(db.kv_get("k").unwrap(), Some(json!(1)));
    }

    #[test]
    fn kv_set_get_delete() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.kv_get("last").unwrap(), None);
        db.kv_set("last", &json!({ "id": "a" })).unwrap();
        db.kv_set("last", &json!({ "id": "b" })).unwrap();
        assert_eq!(db.kv_get("last").unwrap(), Some(json!({ "id": "b" })));
        db.kv_set("last", &Value::Null).unwrap();
        assert_eq!(db.kv_get("last").unwrap(), None);
    }
}
