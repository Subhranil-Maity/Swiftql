//! Runs against temporary database files; no server needed.

use std::{collections::HashMap, path::Path, time::Duration};

use driver_core::{
    connect::{Auth, ConnectConfig, Target, TlsConfig},
    connection::{Connection, ExecOutcome, TxStatus},
    driver::Driver,
    error::ErrorKind,
    introspec::{FkAction, IntrospectScope, SchemaSnapshot, TableKind},
    value::Value,
};
use driver_sqlite::SqliteDriver;

fn config(path: &Path, read_only: bool) -> ConnectConfig {
    let mut options = HashMap::new();
    if read_only {
        options.insert("read_only".to_owned(), "true".to_owned());
    }
    ConnectConfig {
        target: Target::File {
            path: path.to_owned(),
        },
        auth: Auth::None,
        tls: TlsConfig::default(),
        tunnel: None,
        options,
        timeouts: Default::default(),
        host_keys: None,
    }
}

/// A new empty database file, as the app's "New database" button makes it.
fn new_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    std::fs::File::create(&path).unwrap();
    (dir, path)
}

async fn open(path: &Path) -> Box<dyn Connection> {
    SqliteDriver.connect(&config(path, false)).await.unwrap()
}

async fn exec(conn: &dyn Connection, sql: &str) -> ExecOutcome {
    conn.execute(sql)
        .await
        .unwrap_or_else(|e| panic!("{sql}: {e:?}"))
}

async fn rows(conn: &dyn Connection, sql: &str) -> Vec<Vec<Value>> {
    match exec(conn, sql).await {
        ExecOutcome::Rows { mut stream, .. } => {
            let mut out = Vec::new();
            while let Some(batch) = stream.next_batch(2).await.unwrap() {
                out.extend(batch.rows);
                if !batch.has_more {
                    break;
                }
            }
            stream.close().await.unwrap();
            out
        }
        ExecOutcome::Affected { .. } => panic!("{sql}: no rows"),
    }
}

#[tokio::test]
async fn missing_file_is_not_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nope.db");
    let err = SqliteDriver
        .connect(&config(&path, false))
        .await
        .err()
        .expect("missing file");
    assert_eq!(err.kind, ErrorKind::NotFound);
    assert!(!path.exists());
}

#[tokio::test]
async fn not_a_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.txt");
    std::fs::write(&path, "hello, this is not sqlite at all, definitely not").unwrap();
    assert!(SqliteDriver.connect(&config(&path, false)).await.is_err());
}

#[tokio::test]
async fn values_and_counts() {
    let (_dir, path) = new_db();
    let conn = open(&path).await;
    match exec(
        &*conn,
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, ok BOOLEAN, x REAL, b BLOB)",
    )
    .await
    {
        ExecOutcome::Affected { rows, command } => {
            assert_eq!((rows, command.as_str()), (0, "CREATE TABLE"));
        }
        _ => panic!("expected affected"),
    }
    match exec(&*conn, "INSERT INTO t (name, ok, x, b) VALUES ('a', 1, 1.5, x'00ff'), ('b', 0, NULL, NULL), (NULL, NULL, 2, NULL)").await {
        ExecOutcome::Affected { rows, command } => assert_eq!((rows, command.as_str()), (3, "INSERT")),
        _ => panic!("expected affected"),
    }

    let got = rows(
        &*conn,
        "SELECT id, name, ok, x, b, 9007199254740993 FROM t ORDER BY id",
    )
    .await;
    assert_eq!(got.len(), 3);
    assert!(matches!(got[0][0], Value::Int(1)));
    assert!(matches!(&got[0][1], Value::Text(s) if s == "a"));
    assert!(matches!(got[0][2], Value::Bool(true)));
    assert!(matches!(got[0][3], Value::Float(f) if f == 1.5));
    assert!(matches!(&got[0][4], Value::Bytes(b) if b == &[0, 255]));
    assert!(matches!(&got[0][5], Value::BigInt(s) if s == "9007199254740993"));
    assert!(matches!(got[2][1], Value::Null));

    match exec(&*conn, "UPDATE t SET name = 'z' WHERE id > 1").await {
        ExecOutcome::Affected { rows, .. } => assert_eq!(rows, 2),
        _ => panic!("expected affected"),
    }
    match exec(&*conn, "DELETE FROM t RETURNING id").await {
        ExecOutcome::Rows {
            command, stream, ..
        } => {
            assert_eq!(command, "DELETE");
            stream.close().await.unwrap();
        }
        _ => panic!("expected rows"),
    }
}

#[tokio::test]
async fn errors() {
    let (_dir, path) = new_db();
    let conn = open(&path).await;
    let err = conn.execute("SELEC 1").await.err().unwrap();
    assert_eq!(err.kind, ErrorKind::Syntax);
    assert_eq!(err.position, Some(0));
    let err = conn.execute("SELECT * FROM missing").await.err().unwrap();
    assert_eq!(err.kind, ErrorKind::NotFound);
    let err = conn.execute("SELECT 1; SELECT 2").await.err().unwrap();
    assert_eq!(err.kind, ErrorKind::Unsupported);

    exec(&*conn, "CREATE TABLE p (id INTEGER PRIMARY KEY)").await;
    exec(&*conn, "CREATE TABLE c (p INTEGER REFERENCES p)").await;
    let err = conn
        .execute("INSERT INTO c VALUES (5)")
        .await
        .err()
        .unwrap();
    assert_eq!(err.kind, ErrorKind::Constraint, "foreign keys are enforced");
}

#[tokio::test]
async fn transactions() {
    let (_dir, path) = new_db();
    let conn = open(&path).await;
    assert_eq!(conn.tx_status(), TxStatus::Idle);
    exec(&*conn, "BEGIN").await;
    assert_eq!(conn.tx_status(), TxStatus::InTransaction);
    exec(&*conn, "CREATE TABLE t (a)").await;
    assert!(conn.execute("bad sql").await.is_err());
    assert_eq!(conn.tx_status(), TxStatus::InTransaction);
    exec(&*conn, "ROLLBACK").await;
    assert_eq!(conn.tx_status(), TxStatus::Idle);
    assert!(
        conn.execute("SELECT * FROM t").await.is_err(),
        "rolled back"
    );
}

#[tokio::test]
async fn stream_closed_early_frees_the_connection() {
    let (_dir, path) = new_db();
    let conn = open(&path).await;
    let ExecOutcome::Rows { mut stream, .. } = exec(
        &*conn,
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 100000) SELECT i FROM n",
    )
    .await
    else {
        panic!("expected rows")
    };
    let batch = stream.next_batch(10).await.unwrap().unwrap();
    assert_eq!(batch.rows.len(), 10);
    assert!(batch.has_more);
    drop(stream); // without close
    assert_eq!(rows(&*conn, "SELECT 42").await.len(), 1);
}

#[tokio::test]
async fn cancel_and_timeout() {
    let (_dir, path) = new_db();
    let conn = open(&path).await;
    let endless =
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n) SELECT count(*) FROM n";

    let cancel = conn.cancel_handle();
    let canceller = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel.cancel().await.unwrap();
    });
    let err = match conn.execute(endless).await {
        Err(e) => e,
        Ok(ExecOutcome::Rows { mut stream, .. }) => stream.next_batch(10).await.err().unwrap(),
        Ok(_) => panic!("expected rows"),
    };
    canceller.await.unwrap();
    assert_eq!(err.kind, ErrorKind::Cancelled);

    conn.set_query_timeout(1).await.unwrap();
    let err = match conn.execute(endless).await {
        Err(e) => e,
        Ok(ExecOutcome::Rows { mut stream, .. }) => stream.next_batch(10).await.err().unwrap(),
        Ok(_) => panic!("expected rows"),
    };
    assert_eq!(err.kind, ErrorKind::Timeout);
    assert_eq!(rows(&*conn, "SELECT 1").await.len(), 1, "usable afterwards");
}

#[tokio::test]
async fn read_only() {
    let (_dir, path) = new_db();
    let conn = SqliteDriver.connect(&config(&path, true)).await.unwrap();
    let err = conn.execute("CREATE TABLE t (a)").await.err().unwrap();
    assert_eq!(err.kind, ErrorKind::Permission);
}

#[tokio::test]
async fn introspection() {
    let (_dir, path) = new_db();
    let conn = open(&path).await;
    for sql in [
        "CREATE TABLE author (id INTEGER PRIMARY KEY, email TEXT NOT NULL UNIQUE)",
        "CREATE TABLE book (id INTEGER, author_id INTEGER REFERENCES author ON DELETE CASCADE, title TEXT DEFAULT 'x', PRIMARY KEY (id, title))",
        "CREATE INDEX book_title ON book (title)",
        "CREATE VIEW titles AS SELECT title FROM book",
    ] {
        exec(&*conn, sql).await;
    }

    let SchemaSnapshot::Model {
        schemas,
        search_path,
    } = conn.introspect(IntrospectScope::Model).await.unwrap()
    else {
        panic!("expected model")
    };
    assert_eq!(search_path, vec!["main"]);
    assert_eq!(schemas.len(), 1);
    let tables = schemas[0].tables.as_ref().unwrap();
    let names: Vec<_> = tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["author", "book", "titles"]);
    assert_eq!(tables[2].kind, TableKind::View);

    let book = &tables[1];
    assert_eq!(
        book.primary_key.as_deref(),
        Some(&["id".to_owned(), "title".to_owned()][..])
    );
    let cols = book.columns.as_ref().unwrap();
    assert_eq!(cols[2].default.as_deref(), Some("'x'"));
    let fk = &book.foreign_keys.as_ref().unwrap()[0];
    assert_eq!(fk.columns, ["author_id"]);
    assert_eq!(fk.ref_table, "author");
    assert_eq!(
        fk.ref_columns,
        ["id"],
        "resolved from the parent's primary key"
    );
    assert_eq!(fk.on_delete, FkAction::Cascade);
    let indexes = book.indexes.as_ref().unwrap();
    assert!(
        indexes
            .iter()
            .any(|i| i.name == "book_title" && i.definition.starts_with("CREATE INDEX"))
    );
    assert!(indexes.iter().any(|i| i.primary));

    let author = &tables[0];
    let unique = author
        .indexes
        .as_ref()
        .unwrap()
        .iter()
        .find(|i| i.unique && !i.primary)
        .unwrap();
    assert_eq!(unique.columns, ["email"]);
    assert!(!author.columns.as_ref().unwrap()[1].nullable);

    let SchemaSnapshot::Schemas(schemas) = conn.introspect(IntrospectScope::Schemas).await.unwrap()
    else {
        panic!("expected schemas")
    };
    assert_eq!(schemas.len(), 1);
    exec(&*conn, "CREATE TEMP TABLE scratch (a)").await;
    let SchemaSnapshot::Catalog {
        schemas,
        search_path,
    } = conn.introspect(IntrospectScope::Catalog).await.unwrap()
    else {
        panic!("expected catalog")
    };
    assert_eq!(search_path, vec!["main", "temp"]);
    assert_eq!(schemas[1].tables.as_ref().unwrap()[0].name, "scratch");
}
