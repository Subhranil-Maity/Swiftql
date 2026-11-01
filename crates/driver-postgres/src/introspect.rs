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
    }
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
