use driver_core::{
    connection::ServerVersion,
    dbresults::DbResult,
    error::{DbError, ErrorKind},
    introspec::{Column, IntrospectScope, Schema, SchemaSnapshot, Table, TableKind},
};
use tokio_postgres::Client;

use crate::error::map_err;

const SCHEMAS_SQL: &str = r"
    SELECT nspname
    FROM pg_catalog.pg_namespace
    WHERE nspname NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
      AND nspname NOT LIKE 'pg\_temp\_%'
      AND nspname NOT LIKE 'pg\_toast\_temp\_%'
    ORDER BY nspname";

// r table, v view, m materialized view, f foreign table, p partitioned table
const TABLES_SQL: &str = "
    SELECT c.relname, c.relkind::text
    FROM pg_catalog.pg_class c
    JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname = $1
      AND c.relkind IN ('r', 'v', 'm', 'f', 'p')";

// TODO: partitions are hidden from the table list; add some kind of UI for them
const HIDE_PARTITIONS_SQL: &str = " AND NOT c.relispartition"; // column exists in PG 10+

const TABLE_SQL: &str = "
    SELECT c.oid, c.relkind::text
    FROM pg_catalog.pg_class c
    JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname = $1 AND c.relname = $2
      AND c.relkind IN ('r', 'v', 'm', 'f', 'p')";

const COLUMNS_SQL: &str = "
    SELECT a.attname, pg_catalog.format_type(a.atttypid, a.atttypmod), NOT a.attnotnull
    FROM pg_catalog.pg_attribute a
    WHERE a.attrelid = $1 AND a.attnum > 0 AND NOT a.attisdropped
    ORDER BY a.attnum";

// Every user relation with its columns; system schemas are left out.
// TODO: the session's own temp schema (pg_temp_N) is left out too.
const CATALOG_SQL: &str = r"
    SELECT n.nspname, c.relname, c.relkind::text,
           a.attname, pg_catalog.format_type(a.atttypid, a.atttypmod), NOT a.attnotnull
    FROM pg_catalog.pg_namespace n
    LEFT JOIN pg_catalog.pg_class c
      ON c.relnamespace = n.oid AND c.relkind IN ('r', 'v', 'm', 'f', 'p')
    LEFT JOIN pg_catalog.pg_attribute a
      ON a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped
    WHERE n.nspname NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
      AND n.nspname NOT LIKE 'pg\_temp\_%'
      AND n.nspname NOT LIKE 'pg\_toast\_temp\_%'";

const CATALOG_ORDER_SQL: &str = " ORDER BY n.nspname, c.relname, a.attnum";

const SEARCH_PATH_SQL: &str = "SELECT pg_catalog.current_schemas(false)::text[]";

pub async fn introspect(
    client: &Client,
    version: &ServerVersion,
    scope: IntrospectScope,
) -> DbResult<SchemaSnapshot> {
    match scope {
        IntrospectScope::Schemas => {
            let rows = client.query(SCHEMAS_SQL, &[]).await.map_err(map_err)?;
            let schemas = rows
                .iter()
                .map(|row| Schema {
                    name: row.get(0),
                    tables: None,
                })
                .collect();
            Ok(SchemaSnapshot::Schemas(schemas))
        }
        IntrospectScope::Tables { schema } => {
            let mut sql = TABLES_SQL.to_owned();
            if version.at_least(10, 0) {
                sql.push_str(HIDE_PARTITIONS_SQL);
            }
            sql.push_str(" ORDER BY c.relname");

            let rows = client.query(&sql, &[&schema]).await.map_err(map_err)?;
            let tables = rows
                .iter()
                .map(|row| Table {
                    name: row.get(0),
                    kind: table_kind(row.get(1)),
                    columns: None,
                })
                .collect();
            Ok(SchemaSnapshot::Tables(tables))
        }
        IntrospectScope::TableDetails { schema, table } => {
            let row = client
                .query_opt(TABLE_SQL, &[&schema, &table])
                .await
                .map_err(map_err)?
                .ok_or_else(|| {
                    DbError::new(
                        ErrorKind::NotFound,
                        format!("table {schema}.{table} not found"),
                    )
                })?;
            let oid: u32 = row.get(0);
            let kind = table_kind(row.get(1));

            let rows = client.query(COLUMNS_SQL, &[&oid]).await.map_err(map_err)?;
            let columns = rows
                .iter()
                .map(|row| Column {
                    name: row.get(0),
                    data_type: row.get(1),
                    nullable: row.get(2),
                })
                .collect();

            Ok(SchemaSnapshot::TableDetails(Table {
                name: table,
                kind,
                columns: Some(columns),
            }))
        }
        IntrospectScope::Catalog => catalog(client, version).await,
    }
}

async fn catalog(client: &Client, version: &ServerVersion) -> DbResult<SchemaSnapshot> {
    let mut sql = CATALOG_SQL.to_owned();
    if version.at_least(10, 0) {
        sql.push_str(" AND (c.oid IS NULL OR NOT c.relispartition)");
    }
    sql.push_str(CATALOG_ORDER_SQL);
    let rows = client.query(&sql, &[]).await.map_err(map_err)?;

    // Rows come sorted by schema, table, column: group them as they arrive.
    let mut schemas: Vec<Schema> = Vec::new();
    for row in &rows {
        let schema: String = row.get(0);
        if schemas.last().is_none_or(|s| s.name != schema) {
            schemas.push(Schema {
                name: schema,
                tables: Some(Vec::new()),
            });
        }
        let tables = schemas.last_mut().unwrap().tables.get_or_insert_default();

        let Some(table) = row.get::<_, Option<String>>(1) else {
            continue; // empty schema
        };
        if tables.last().is_none_or(|t| t.name != table) {
            tables.push(Table {
                name: table,
                kind: table_kind(row.get(2)),
                columns: Some(Vec::new()),
            });
        }
        let Some(column) = row.get::<_, Option<String>>(3) else {
            continue; // no columns
        };
        let columns = tables.last_mut().unwrap().columns.get_or_insert_default();
        columns.push(Column {
            name: column,
            data_type: row.get(4),
            nullable: row.get(5),
        });
    }

    let search_path: Vec<String> = client
        .query_one(SEARCH_PATH_SQL, &[])
        .await
        .map_err(map_err)?
        .get(0);
    Ok(SchemaSnapshot::Catalog {
        schemas,
        search_path,
    })
}

/// pg_class.relkind → TableKind. Only called with the kinds the queries select.
fn table_kind(relkind: &str) -> TableKind {
    match relkind {
        "v" => TableKind::View,
        "m" => TableKind::MaterializedView,
        "f" => TableKind::ForeignTable,
        "p" => TableKind::PartitionedTable,
        _ => TableKind::Table,
    }
}
