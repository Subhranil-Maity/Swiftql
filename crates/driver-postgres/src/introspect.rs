use std::collections::HashMap;

use driver_core::{
    connection::ServerVersion,
    dbresults::DbResult,
    error::{DbError, ErrorKind},
    introspec::{
        Column, FkAction, ForeignKey, Index, IntrospectScope, Schema, SchemaSnapshot, Table,
        TableKind,
    },
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

/// Leaves out system schemas; `n` is pg_namespace.
// TODO: the session's own temp schema (pg_temp_N) is left out too.
const USER_SCHEMAS: &str = r"
      n.nspname NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
      AND n.nspname NOT LIKE 'pg\_temp\_%'
      AND n.nspname NOT LIKE 'pg\_toast\_temp\_%'";

// Every user relation with its columns. `{extra}` adds the Model scope's
// column comment and default; `{join}` the table the default comes from.
const CATALOG_SQL: &str = "
    SELECT n.nspname, c.relname, c.relkind::text,
           a.attname, pg_catalog.format_type(a.atttypid, a.atttypmod), NOT a.attnotnull{extra}
    FROM pg_catalog.pg_namespace n
    LEFT JOIN pg_catalog.pg_class c
      ON c.relnamespace = n.oid AND c.relkind IN ('r', 'v', 'm', 'f', 'p')
    LEFT JOIN pg_catalog.pg_attribute a
      ON a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped{join}
    WHERE {schemas}";

const COLUMN_EXTRA_SQL: &str = ",
           pg_catalog.col_description(a.attrelid, a.attnum),
           pg_catalog.pg_get_expr(d.adbin, d.adrelid)";

const COLUMN_DEFAULT_JOIN_SQL: &str = "
    LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum";

const CATALOG_ORDER_SQL: &str = " ORDER BY n.nspname, c.relname, a.attnum";

const SEARCH_PATH_SQL: &str = "SELECT pg_catalog.current_schemas(false)::text[]";

// Comment, row estimate and size per relation. reltuples is -1 until the
// first ANALYZE (PG 14+). A partitioned table holds no rows itself, so its
// direct partitions are added up.
// TODO: nested partitions (partitions that are partitioned again) aren't counted
const RELATIONS_SQL: &str = "
    SELECT n.nspname, c.relname,
           pg_catalog.obj_description(c.oid, 'pg_class'),
           CASE WHEN c.relkind = 'p' THEN parts.row_estimate
                WHEN c.reltuples < 0 THEN NULL
                ELSE c.reltuples::int8 END,
           CASE WHEN c.relkind = 'p' THEN parts.size_bytes
                WHEN c.relkind IN ('r', 'm') THEN pg_catalog.pg_total_relation_size(c.oid) END
    FROM pg_catalog.pg_class c
    JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
    LEFT JOIN LATERAL (
        SELECT sum(pg_catalog.pg_total_relation_size(i.inhrelid))::int8 AS size_bytes,
               sum(GREATEST(ch.reltuples, 0))::int8 AS row_estimate
        FROM pg_catalog.pg_inherits i
        JOIN pg_catalog.pg_class ch ON ch.oid = i.inhrelid
        WHERE i.inhparent = c.oid
    ) parts ON c.relkind = 'p'
    WHERE c.relkind IN ('r', 'v', 'm', 'f', 'p') AND {schemas}";

/// Column names of attnum array `{keys}` on relation `{rel}`; `WITH
/// ORDINALITY` keeps the key's order. Expression entries (attnum 0) drop out.
const KEY_COLUMNS_SQL: &str = "
           ARRAY(SELECT a.attname::text
                 FROM unnest({keys}) WITH ORDINALITY k(attnum, ord)
                 JOIN pg_catalog.pg_attribute a ON a.attrelid = {rel} AND a.attnum = k.attnum
                 ORDER BY k.ord)";

// Primary and foreign keys. confrelid is 0 for primary keys, so the
// referenced side comes out NULL.
const KEYS_SQL: &str = "
    SELECT n.nspname, c.relname, con.conname, con.contype::text,
           {columns}, rn.nspname, rc.relname, {ref_columns},
           con.confdeltype::text, con.confupdtype::text
    FROM pg_catalog.pg_constraint con
    JOIN pg_catalog.pg_class c ON c.oid = con.conrelid
    JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
    LEFT JOIN pg_catalog.pg_class rc ON rc.oid = con.confrelid
    LEFT JOIN pg_catalog.pg_namespace rn ON rn.oid = rc.relnamespace
    WHERE con.contype IN ('p', 'f') AND {schemas}";

// PG 11+: a foreign key to a partitioned table gets a copy per partition
// (conparentid points at the original); only the original is shown.
const TOP_LEVEL_CONSTRAINTS_SQL: &str = " AND con.conparentid = 0";

// TODO: INCLUDE columns are listed with the key columns (indnkeyatts is PG 11+)
const INDEXES_SQL: &str = "
    SELECT n.nspname, c.relname, ic.relname, i.indisunique, i.indisprimary,
           {columns}, pg_catalog.pg_get_indexdef(i.indexrelid)
    FROM pg_catalog.pg_index i
    JOIN pg_catalog.pg_class c ON c.oid = i.indrelid
    JOIN pg_catalog.pg_class ic ON ic.oid = i.indexrelid
    JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
    WHERE c.relkind IN ('r', 'm', 'p') AND {schemas}";

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
                    ..Table::default()
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
                    ..Column::default()
                })
                .collect();

            Ok(SchemaSnapshot::TableDetails(Table {
                name: table,
                kind,
                columns: Some(columns),
                ..Table::default()
            }))
        }
        IntrospectScope::Catalog => Ok(SchemaSnapshot::Catalog {
            schemas: load_schemas(client, version, false).await?,
            search_path: search_path(client).await?,
        }),
        IntrospectScope::Model => model(client, version).await,
    }
}

/// Every user schema with its tables and columns. `detailed` adds column
/// comments and defaults.
async fn load_schemas(
    client: &Client,
    version: &ServerVersion,
    detailed: bool,
) -> DbResult<Vec<Schema>> {
    let mut sql = CATALOG_SQL
        .replace("{extra}", if detailed { COLUMN_EXTRA_SQL } else { "" })
        .replace(
            "{join}",
            if detailed {
                COLUMN_DEFAULT_JOIN_SQL
            } else {
                ""
            },
        )
        .replace("{schemas}", USER_SCHEMAS);
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
                ..Table::default()
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
            comment: if detailed { row.get(6) } else { None },
            default: if detailed { row.get(7) } else { None },
        });
    }
    Ok(schemas)
}

async fn search_path(client: &Client) -> DbResult<Vec<String>> {
    Ok(client
        .query_one(SEARCH_PATH_SQL, &[])
        .await
        .map_err(map_err)?
        .get(0))
}

/// What the Model scope adds to one table.
#[derive(Default)]
struct TableExtras {
    comment: Option<String>,
    row_estimate: Option<i64>,
    size_bytes: Option<i64>,
    primary_key: Vec<String>,
    foreign_keys: Vec<ForeignKey>,
    indexes: Vec<Index>,
}

/// The catalog plus comments, sizes, keys and indexes. Each part is one
/// query, joined to the tables here by (schema, table).
async fn model(client: &Client, version: &ServerVersion) -> DbResult<SchemaSnapshot> {
    let mut schemas = load_schemas(client, version, true).await?;
    let hide_partitions = version.at_least(10, 0);
    let filtered = |sql: String| {
        let mut sql = sql.replace("{schemas}", USER_SCHEMAS);
        if hide_partitions {
            sql.push_str(HIDE_PARTITIONS_SQL);
        }
        sql
    };
    let mut extras: HashMap<(String, String), TableExtras> = HashMap::new();

    let rows = client
        .query(&filtered(RELATIONS_SQL.to_owned()), &[])
        .await
        .map_err(map_err)?;
    for row in &rows {
        let entry = extras.entry((row.get(0), row.get(1))).or_default();
        entry.comment = row.get(2);
        entry.row_estimate = row.get(3);
        entry.size_bytes = row.get(4);
    }

    let mut sql = filtered(
        KEYS_SQL
            .replace("{columns}", &key_columns("con.conkey", "con.conrelid"))
            .replace(
                "{ref_columns}",
                &key_columns("con.confkey", "con.confrelid"),
            ),
    );
    if version.at_least(11, 0) {
        sql.push_str(TOP_LEVEL_CONSTRAINTS_SQL);
    }
    sql.push_str(" ORDER BY n.nspname, c.relname, con.conname");
    let rows = client.query(&sql, &[]).await.map_err(map_err)?;
    for row in &rows {
        let entry = extras.entry((row.get(0), row.get(1))).or_default();
        let columns: Vec<String> = row.get(4);
        if row.get::<_, &str>(3) == "p" {
            entry.primary_key = columns;
            continue;
        }
        let (Some(ref_schema), Some(ref_table)) = (row.get(5), row.get(6)) else {
            continue; // the referenced table vanished mid-read
        };
        entry.foreign_keys.push(ForeignKey {
            name: row.get(2),
            columns,
            ref_schema,
            ref_table,
            ref_columns: row.get(7),
            on_delete: fk_action(row.get(8)),
            on_update: fk_action(row.get(9)),
        });
    }

    let mut sql =
        filtered(INDEXES_SQL.replace("{columns}", &key_columns("i.indkey::int2[]", "i.indrelid")));
    sql.push_str(" ORDER BY n.nspname, c.relname, i.indisprimary DESC, ic.relname");
    let rows = client.query(&sql, &[]).await.map_err(map_err)?;
    for row in &rows {
        let entry = extras.entry((row.get(0), row.get(1))).or_default();
        entry.indexes.push(Index {
            name: row.get(2),
            unique: row.get(3),
            primary: row.get(4),
            columns: row.get(5),
            definition: row.get(6),
        });
    }

    for schema in &mut schemas {
        for table in schema.tables.iter_mut().flatten() {
            let found = extras
                .remove(&(schema.name.clone(), table.name.clone()))
                .unwrap_or_default();
            table.comment = found.comment;
            table.row_estimate = found.row_estimate;
            table.size_bytes = found.size_bytes;
            table.primary_key = Some(found.primary_key);
            table.foreign_keys = Some(found.foreign_keys);
            table.indexes = Some(found.indexes);
        }
    }

    Ok(SchemaSnapshot::Model {
        schemas,
        search_path: search_path(client).await?,
    })
}

/// SQL for the column names of attnum array `keys` on relation `rel`.
fn key_columns(keys: &str, rel: &str) -> String {
    KEY_COLUMNS_SQL
        .replace("{keys}", keys)
        .replace("{rel}", rel)
}

/// pg_constraint.confdeltype / confupdtype → FkAction.
fn fk_action(code: &str) -> FkAction {
    match code {
        "r" => FkAction::Restrict,
        "c" => FkAction::Cascade,
        "n" => FkAction::SetNull,
        "d" => FkAction::SetDefault,
        _ => FkAction::NoAction, // "a"
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
