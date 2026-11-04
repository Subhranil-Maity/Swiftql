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
    let catalog = conn.introspect(IntrospectScope::Catalog).await;
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

    let Ok(SchemaSnapshot::Catalog {
        schemas,
        search_path,
    }) = catalog
    else {
        panic!("expected a catalog: {catalog:?}");
    };
    assert!(!search_path.is_empty());
    let ours = schemas.iter().find(|s| s.name == schema).unwrap();
    let tables = ours.tables.as_ref().unwrap();
    let names: Vec<_> = tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["events", "user_emails", "users"]);
    let users = tables.iter().find(|t| t.name == "users").unwrap();
    let columns: Vec<_> = users
        .columns
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(columns, ["id", "email"]);
    assert!(schemas.iter().all(|s| s.name != "pg_catalog"));
}

/// Two tables linked by a foreign key, with comments, a default and an index.
#[tokio::test]
#[ignore]
async fn introspect_model() {
    use driver_core::introspec::FkAction;

    let schema = format!("swiftql_model_{}", std::process::id());
    let (setup, connection) = tokio_postgres::connect(&url(), tokio_postgres::NoTls)
        .await
        .unwrap();
    tokio::spawn(connection);
    setup
        .batch_execute(&format!(
            "CREATE SCHEMA {schema};
             CREATE TABLE {schema}.customers (id bigserial PRIMARY KEY, email text NOT NULL);
             CREATE TABLE {schema}.orders (
                 id bigint,
                 line int,
                 customer_id bigint REFERENCES {schema}.customers (id) ON DELETE CASCADE,
                 status text DEFAULT 'new',
                 PRIMARY KEY (line, id)
             );
             CREATE INDEX orders_status_idx ON {schema}.orders (status, customer_id);
             COMMENT ON TABLE {schema}.orders IS 'One row per order line';
             COMMENT ON COLUMN {schema}.customers.email IS 'Login address';
             INSERT INTO {schema}.customers (email) VALUES ('a@b.c');
             ANALYZE {schema}.customers;"
        ))
        .await
        .unwrap();

    let driver = PgDriver;
    let conn = driver
        .connect(&driver.parse_url(&url()).unwrap())
        .await
        .unwrap();
    let model = conn.introspect(IntrospectScope::Model).await;
    conn.close().await;
    setup
        .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
        .await
        .unwrap();

    let Ok(SchemaSnapshot::Model { schemas, .. }) = model else {
        panic!("expected a model: {model:?}");
    };
    let tables = schemas
        .iter()
        .find(|s| s.name == schema)
        .and_then(|s| s.tables.as_ref())
        .unwrap();
    let customers = tables.iter().find(|t| t.name == "customers").unwrap();
    let orders = tables.iter().find(|t| t.name == "orders").unwrap();

    assert_eq!(
        customers.primary_key.as_deref(),
        Some(&["id".to_owned()][..])
    );
    assert_eq!(customers.row_estimate, Some(1));
    assert!(customers.size_bytes.unwrap() > 0);
    let columns = customers.columns.as_ref().unwrap();
    assert_eq!(columns[1].comment.as_deref(), Some("Login address"));
    assert!(
        columns[0]
            .default
            .as_deref()
            .unwrap()
            .starts_with("nextval(")
    );
    assert_eq!(customers.foreign_keys.as_ref().map(Vec::len), Some(0));

    // Composite key in declared order, not column order.
    assert_eq!(
        orders.primary_key.as_deref(),
        Some(&["line".to_owned(), "id".to_owned()][..])
    );
    assert_eq!(orders.comment.as_deref(), Some("One row per order line"));
    let status = &orders.columns.as_ref().unwrap()[3];
    assert_eq!(status.default.as_deref(), Some("'new'::text"));

    let fks = orders.foreign_keys.as_ref().unwrap();
    assert_eq!(fks.len(), 1);
    assert_eq!(fks[0].columns, ["customer_id"]);
    assert_eq!(fks[0].ref_schema, schema);
    assert_eq!(fks[0].ref_table, "customers");
    assert_eq!(fks[0].ref_columns, ["id"]);
    assert_eq!(fks[0].on_delete, FkAction::Cascade);
    assert_eq!(fks[0].on_update, FkAction::NoAction);

    let indexes = orders.indexes.as_ref().unwrap();
    assert_eq!(indexes.len(), 2);
    assert!(indexes[0].primary && indexes[0].unique);
    assert_eq!(indexes[1].name, "orders_status_idx");
    assert_eq!(indexes[1].columns, ["status", "customer_id"]);
    assert!(!indexes[1].unique);
    assert!(
        indexes[1]
            .definition
            .starts_with("CREATE INDEX orders_status_idx")
    );
}
