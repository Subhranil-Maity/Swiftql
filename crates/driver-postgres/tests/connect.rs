//! Needs a running server. Run with:
//! SWIFTQL_PG_URL=postgres://user:pass@localhost/db cargo test -p driver-postgres -- --ignored

use driver_core::{
    driver::Driver,
    error::ErrorKind,
    introspec::{IntrospectScope, SchemaSnapshot},
};
use driver_postgres::PgDriver;

fn url() -> String {
    std::env::var("SWIFTQL_PG_URL").expect("set SWIFTQL_PG_URL to run these tests")
}

#[tokio::test]
#[ignore]
async fn connect_ping_introspect() {
    let driver = PgDriver;
    let config = driver.parse_url(&url()).unwrap();
    driver.test_conn(&config).await.unwrap();

    let conn = driver.connect(&config).await.unwrap();
    assert!(conn.server_version().major >= 10);
    conn.ping().await.unwrap();

    let SchemaSnapshot::Schemas(schemas) = conn.introspect(IntrospectScope::Schemas).await.unwrap()
    else {
        panic!("expected schemas");
    };
    assert!(schemas.iter().all(|s| s.name != "pg_catalog"));
    conn.close().await;
}

#[tokio::test]
#[ignore]
async fn wrong_password_is_auth_error() {
    let driver = PgDriver;
    let mut config = driver.parse_url(&url()).unwrap();
    if let driver_core::connect::Auth::Password { user, .. } = &config.auth {
        config.auth = driver_core::connect::Auth::Password {
            user: user.clone(),
            password: driver_core::connect::Secret::new("definitely-wrong"),
        };
    }
    let err = driver
        .connect(&config)
        .await
        .err()
        .expect("connect should fail");
    assert_eq!(err.kind, ErrorKind::Auth);
}

/// Creates a scratch schema with a table, view and partitioned table, then drops it.
#[tokio::test]
#[ignore]
async fn introspect_tables_and_details() {
    use driver_core::introspec::TableKind;

    let schema = format!("swiftql_test_{}", std::process::id());
    let (setup, connection) = tokio_postgres::connect(&url(), tokio_postgres::NoTls)
        .await
        .unwrap();
    tokio::spawn(connection);
    setup
        .batch_execute(&format!(
            "CREATE SCHEMA {schema};
             CREATE TABLE {schema}.users (id int PRIMARY KEY, email varchar(255));
             CREATE VIEW {schema}.user_emails AS SELECT email FROM {schema}.users;
             CREATE TABLE {schema}.events (at date) PARTITION BY RANGE (at);
             CREATE TABLE {schema}.events_2026 PARTITION OF {schema}.events
                 FOR VALUES FROM ('2026-01-01') TO ('2027-01-01');"
        ))
        .await
        .unwrap();

    let driver = PgDriver;
    let conn = driver
        .connect(&driver.parse_url(&url()).unwrap())
        .await
        .unwrap();
    let tables = conn
        .introspect(IntrospectScope::Tables {
            schema: schema.clone(),
        })
        .await;
    let details = conn
        .introspect(IntrospectScope::TableDetails {
            schema: schema.clone(),
            table: "users".into(),
        })
        .await;
    let missing = conn
        .introspect(IntrospectScope::TableDetails {
            schema: schema.clone(),
            table: "nope".into(),
        })
        .await;
    conn.close().await;
    setup
        .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
        .await
        .unwrap();

    let Ok(SchemaSnapshot::Tables(tables)) = tables else {
        panic!("expected tables: {tables:?}");
    };
    let listed: Vec<_> = tables.iter().map(|t| (t.name.as_str(), t.kind)).collect();
    assert_eq!(
        listed,
        [
            ("events", TableKind::PartitionedTable), // events_2026 is hidden
            ("user_emails", TableKind::View),
            ("users", TableKind::Table),
        ]
    );

    let Ok(SchemaSnapshot::TableDetails(users)) = details else {
        panic!("expected table details: {details:?}");
    };
    let columns = users.columns.unwrap();
    assert_eq!(columns[0].name, "id");
    assert_eq!(columns[0].data_type, "integer");
    assert!(!columns[0].nullable);
    assert_eq!(columns[1].data_type, "character varying(255)");
    assert!(columns[1].nullable);

    assert_eq!(missing.err().map(|e| e.kind), Some(ErrorKind::NotFound));
}
