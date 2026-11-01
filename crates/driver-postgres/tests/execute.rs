//! Needs a running server. Run with:
//! SWIFTQL_PG_URL=postgres://user:pass@localhost/db cargo test -p driver-postgres -- --ignored

use driver_core::{
    connection::{Connection, ExecOutcome, RowBatch, TxStatus},
    driver::Driver,
    error::ErrorKind,
    types::LogicalType,
    value::Value,
};
use driver_postgres::PgDriver;

async fn connect() -> Box<dyn Connection> {
    let url = std::env::var("SWIFTQL_PG_URL").expect("set SWIFTQL_PG_URL to run these tests");
    let driver = PgDriver;
    driver
        .connect(&driver.parse_url(&url).unwrap())
        .await
        .unwrap()
}

/// Run a statement that must return rows; read everything in one batch.
async fn rows(conn: &dyn Connection, sql: &str) -> Vec<Vec<Value>> {
    match conn.execute(sql).await.unwrap() {
        ExecOutcome::Rows { mut stream, .. } => {
            let batch = stream.next_batch(10_000).await.unwrap();
            stream.close().await.unwrap();
            batch.map(|b| b.rows).unwrap_or_default()
        }
        ExecOutcome::Affected { .. } => panic!("expected rows for {sql}"),
    }
}

async fn affected(conn: &dyn Connection, sql: &str) -> (u64, String) {
    match conn.execute(sql).await.unwrap() {
        ExecOutcome::Affected { rows, command } => (rows, command),
        ExecOutcome::Rows { .. } => panic!("expected affected rows for {sql}"),
    }
}

#[tokio::test]
#[ignore]
async fn decodes_types() {
    let conn = connect().await;
    let ExecOutcome::Rows { columns, mut stream } = conn
        .execute(
            "SELECT 1::int4, 9007199254740993::int8, 12.340::numeric, 'hi'::text, NULL::text,
                    '{\"a\": [1, 2]}'::jsonb, ARRAY[[1, 2], [3, 4]], int4range(1, 5),
                    '2026-01-02 03:04:05+00'::timestamptz, '1 mon 2 days 3 s'::interval,
                    '10.0.0.1/8'::inet, 'NaN'::float8, true, '00000000-0000-0000-0000-000000000001'::uuid",
        )
        .await
        .unwrap()
    else {
        panic!("expected rows");
    };
    assert_eq!(columns[0].logical, LogicalType::Int);
    assert_eq!(
        columns[6].logical,
        LogicalType::Array(Box::new(LogicalType::Int))
    );

    let RowBatch { rows, has_more } = stream.next_batch(10).await.unwrap().unwrap();
    assert!(!has_more);
    let row = &rows[0];
    assert!(matches!(row[0], Value::Int(1)));
    assert!(matches!(&row[1], Value::BigInt(s) if s == "9007199254740993"));
    assert!(matches!(&row[2], Value::Decimal(s) if s == "12.340"));
    assert!(matches!(&row[3], Value::Text(s) if s == "hi"));
    assert!(matches!(row[4], Value::Null));
    assert!(matches!(&row[5], Value::Json(j) if j["a"][1] == 2));
    assert!(matches!(&row[6], Value::Array(outer) if outer.len() == 2));
    assert!(matches!(
        &row[7],
        Value::Range {
            lower_inclusive: true,
            upper_inclusive: false,
            ..
        }
    ));
    assert!(
        matches!(&row[8], Value::TimestampTz(t) if t.to_rfc3339() == "2026-01-02T03:04:05+00:00")
    );
    assert!(matches!(
        row[9],
        Value::Interval {
            months: 1,
            days: 2,
            microseconds: 3_000_000
        }
    ));
    assert!(matches!(row[10], Value::Inet { prefix: 8, .. }));
    assert!(matches!(&row[11], Value::Unknown { text, .. } if text == "NaN"));
    assert!(matches!(row[12], Value::Bool(true)));
    assert!(matches!(row[13], Value::Uuid(_)));
    assert!(stream.next_batch(10).await.unwrap().is_none());
}

#[tokio::test]
#[ignore]
async fn batches_report_has_more() {
    let conn = connect().await;
    let ExecOutcome::Rows { mut stream, .. } =
        conn.execute("SELECT generate_series(1, 25)").await.unwrap()
    else {
        panic!("expected rows");
    };
    let mut sizes = Vec::new();
    while let Some(batch) = stream.next_batch(10).await.unwrap() {
        sizes.push((batch.rows.len(), batch.has_more));
    }
    assert_eq!(sizes, [(10, true), (10, true), (5, false)]);
}

#[tokio::test]
#[ignore]
async fn affected_rows_and_tags() {
    let conn = connect().await;
    let (_, tag) = affected(&*conn, "CREATE TEMP TABLE t (id int, name text)").await;
    assert_eq!(tag, "CREATE TABLE");
    let (n, tag) = affected(&*conn, "INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')").await;
    assert_eq!((n, tag.as_str()), (3, "INSERT"));
    let (n, tag) = affected(&*conn, "UPDATE t SET name = 'x' WHERE id > 1").await;
    assert_eq!((n, tag.as_str()), (2, "UPDATE"));

    // RETURNING makes it a row-returning statement
    let returned = rows(&*conn, "DELETE FROM t WHERE id = 1 RETURNING id").await;
    assert!(matches!(returned[..], [ref r] if matches!(r[0], Value::Int(1))));
}

#[tokio::test]
#[ignore]
async fn rejects_multi_statement_and_params() {
    let conn = connect().await;
    let err = conn.execute("SELECT 1; SELECT 2").await.err().unwrap();
    assert_eq!(err.kind, ErrorKind::Unsupported);
    let err = conn.execute("SELECT $1::int").await.err().unwrap();
    assert_eq!(err.kind, ErrorKind::Unsupported);
    let err = conn.execute("SELEC 1").await.err().unwrap();
    assert_eq!((err.kind, err.position), (ErrorKind::Syntax, Some(0)));
}

#[tokio::test]
#[ignore]
async fn tracks_transaction_status() {
    let conn = connect().await;
    assert_eq!(conn.tx_status(), TxStatus::Idle);

    affected(&*conn, "BEGIN").await;
    assert_eq!(conn.tx_status(), TxStatus::InTransaction);
    rows(&*conn, "SELECT 1").await;
    assert_eq!(conn.tx_status(), TxStatus::InTransaction);

    assert!(conn.execute("SELECT * FROM no_such_table").await.is_err());
    assert_eq!(conn.tx_status(), TxStatus::Failed);
    assert!(conn.execute("SELECT 1").await.is_err()); // 25P02 until rollback
    assert_eq!(conn.tx_status(), TxStatus::Failed);

    affected(&*conn, "ROLLBACK").await;
    assert_eq!(conn.tx_status(), TxStatus::Idle);

    // savepoints recover a failed transaction
    affected(&*conn, "BEGIN").await;
    affected(&*conn, "SAVEPOINT sp").await;
    assert!(conn.execute("SELECT 1/0").await.is_err());
    assert_eq!(conn.tx_status(), TxStatus::Failed);
    affected(&*conn, "ROLLBACK TO SAVEPOINT sp").await;
    assert_eq!(conn.tx_status(), TxStatus::InTransaction);
    affected(&*conn, "COMMIT").await;
    assert_eq!(conn.tx_status(), TxStatus::Idle);
}

#[tokio::test]
#[ignore]
async fn error_while_streaming_fails_transaction() {
    let conn = connect().await;
    affected(&*conn, "BEGIN").await;
    let ExecOutcome::Rows { mut stream, .. } = conn
        .execute("SELECT 1 / (x - 3) FROM generate_series(1, 5) x")
        .await
        .unwrap()
    else {
        panic!("expected rows");
    };
    let err = stream.next_batch(10).await.err().unwrap();
    assert_eq!(err.code.as_deref(), Some("22012")); // division_by_zero
    assert_eq!(conn.tx_status(), TxStatus::Failed);
    affected(&*conn, "ROLLBACK").await;
}

#[tokio::test]
#[ignore]
async fn early_close_cancels_and_frees_connection() {
    let conn = connect().await;
    let ExecOutcome::Rows { mut stream, .. } = conn
        .execute("SELECT generate_series(1, 50000000)")
        .await
        .unwrap()
    else {
        panic!("expected rows");
    };
    assert!(stream.next_batch(100).await.unwrap().unwrap().has_more);

    let started = std::time::Instant::now();
    stream.close().await.unwrap();
    assert!(
        started.elapsed().as_secs() < 5,
        "close took {:?}",
        started.elapsed()
    );

    assert_eq!(rows(&*conn, "SELECT 42").await.len(), 1); // connection still usable
}
