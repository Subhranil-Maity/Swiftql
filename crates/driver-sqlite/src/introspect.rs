//! Schemas are the connection's databases: main, temp (when it holds
//! anything) and ATTACHed ones. Tables, columns, keys and indexes come from
//! sqlite_schema and the table-valued PRAGMA functions.

use driver_core::{
    dbresults::DbResult,
    dialect::Dialect,
    error::{DbError, ErrorKind},
    introspec::{
        Column, FkAction, ForeignKey, Index, IntrospectScope, Schema, SchemaSnapshot, Table,
        TableKind,
    },
};
use rusqlite::{Connection, params};

use crate::{dialect::SqliteDialect, error::map_err};

fn err(e: rusqlite::Error) -> DbError {
    map_err(e, false)
}

pub fn introspect(conn: &Connection, scope: IntrospectScope) -> DbResult<SchemaSnapshot> {
    match scope {
        IntrospectScope::Schemas => Ok(SchemaSnapshot::Schemas(
            schema_names(conn)?
                .into_iter()
                .map(|name| Schema { name, tables: None })
                .collect(),
        )),
        IntrospectScope::Tables { schema } => {
            let tables = tables(conn, &schema)?
                .into_iter()
                .map(|(name, kind)| Table {
                    name,
                    kind,
                    ..Table::default()
                })
                .collect();
            Ok(SchemaSnapshot::Tables(tables))
        }
        IntrospectScope::TableDetails { schema, table } => {
            let kind = tables(conn, &schema)?
                .into_iter()
                .find(|(name, _)| *name == table)
                .map(|(_, kind)| kind)
                .ok_or_else(|| {
                    DbError::new(
                        ErrorKind::NotFound,
                        format!("table {schema}.{table} does not exist"),
                    )
                })?;
            Ok(SchemaSnapshot::TableDetails(table_model(
                conn, &schema, table, kind,
            )?))
        }
        IntrospectScope::Catalog => {
            let (schemas, search_path) = catalog(conn, false)?;
            Ok(SchemaSnapshot::Catalog {
                schemas,
                search_path,
            })
        }
        IntrospectScope::Model => {
            let (schemas, search_path) = catalog(conn, true)?;
            Ok(SchemaSnapshot::Model {
                schemas,
                search_path,
            })
        }
    }
}

/// Every schema with its tables; with `model`, keys and indexes too.
/// The search path is main, temp, then attached ones in attach order
/// (`PRAGMA database_list` order). main comes first because new tables go
/// there and the console completes against the first entry.
// TODO: SQLite actually looks in temp before main, so a temp table shadows
// a main one of the same name.
fn catalog(conn: &Connection, model: bool) -> DbResult<(Vec<Schema>, Vec<String>)> {
    let names = schema_names(conn)?;
    let mut schemas = Vec::with_capacity(names.len());
    for name in &names {
        let mut list = Vec::new();
        for (table, kind) in tables(conn, name)? {
            list.push(if model {
                table_model(conn, name, table, kind)?
            } else {
                Table {
                    columns: Some(columns(conn, name, &table)?),
                    name: table,
                    kind,
                    ..Table::default()
                }
            });
        }
        schemas.push(Schema {
            name: name.clone(),
            tables: Some(list),
        });
    }
    Ok((schemas, names))
}

/// main, temp if it holds anything, then attached databases.
fn schema_names(conn: &Connection) -> DbResult<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT name FROM pragma_database_list ORDER BY seq")
        .map_err(err)?;
    let all: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)?;
    let temp_used: bool = conn
        .query_row("SELECT count(*) > 0 FROM temp.sqlite_schema", [], |row| {
            row.get(0)
        })
        .map_err(err)?;
    Ok(all
        .into_iter()
        .filter(|name| name != "temp" || temp_used)
        .collect())
}

/// Tables and views of a schema by name, leaving out SQLite's own.
fn tables(conn: &Connection, schema: &str) -> DbResult<Vec<(String, TableKind)>> {
    let sql = format!(
        r"SELECT name, type FROM {}.sqlite_schema
          WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite\_%' ESCAPE '\'
          ORDER BY name",
        SqliteDialect.quote_ident(schema)
    );
    let mut stmt = conn.prepare(&sql).map_err(err)?;
    let rows = stmt
        .query_map([], |row| {
            let kind: String = row.get(1)?;
            Ok((
                row.get(0)?,
                if kind == "view" {
                    TableKind::View
                } else {
                    TableKind::Table
                },
            ))
        })
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)?;
    Ok(rows)
}

struct ColumnRow {
    column: Column,
    pk: i64, // position in the primary key, 0 = not part of it
}

fn column_rows(conn: &Connection, schema: &str, table: &str) -> DbResult<Vec<ColumnRow>> {
    // hidden: 0 normal, 1 hidden column of a virtual table, 2/3 generated
    let mut stmt = conn
        .prepare(
            r#"SELECT name, type, "notnull", dflt_value, pk
               FROM pragma_table_xinfo(?1, ?2) WHERE hidden <> 1 ORDER BY cid"#,
        )
        .map_err(err)?;
    let rows = stmt
        .query_map(params![table, schema], |row| {
            let data_type: String = row.get(1)?;
            let not_null: bool = row.get(2)?;
            Ok(ColumnRow {
                column: Column {
                    name: row.get(0)?,
                    data_type: if data_type.is_empty() {
                        "any".into()
                    } else {
                        data_type
                    },
                    nullable: !not_null,
                    comment: None,
                    default: row.get(3)?,
                },
                pk: row.get(4)?,
            })
        })
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)?;
    Ok(rows)
}

fn columns(conn: &Connection, schema: &str, table: &str) -> DbResult<Vec<Column>> {
    Ok(column_rows(conn, schema, table)?
        .into_iter()
        .map(|c| c.column)
        .collect())
}

fn primary_key(rows: &[ColumnRow]) -> Vec<String> {
    let mut pk: Vec<_> = rows.iter().filter(|c| c.pk > 0).collect();
    pk.sort_by_key(|c| c.pk);
    pk.into_iter().map(|c| c.column.name.clone()).collect()
}

/// A table with columns, primary key, foreign keys and indexes.
fn table_model(conn: &Connection, schema: &str, name: String, kind: TableKind) -> DbResult<Table> {
    let rows = column_rows(conn, schema, &name)?;
    let primary_key = primary_key(&rows);
    let (foreign_keys, indexes) = if kind == TableKind::Table {
        (
            foreign_keys(conn, schema, &name)?,
            indexes(conn, schema, &name)?,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(Table {
        columns: Some(rows.into_iter().map(|c| c.column).collect()),
        primary_key: Some(primary_key),
        foreign_keys: Some(foreign_keys),
        indexes: Some(indexes),
        // TODO: sizes need the dbstat virtual table, which builds may leave out;
        // no row estimate without ANALYZE (sqlite_stat1).
        size_bytes: None,
        row_estimate: None,
        comment: None,
        name,
        kind,
    })
}

fn foreign_keys(conn: &Connection, schema: &str, table: &str) -> DbResult<Vec<ForeignKey>> {
    let mut stmt = conn
        .prepare(
            r#"SELECT id, "table", "from", "to", on_update, on_delete
               FROM pragma_foreign_key_list(?1, ?2) ORDER BY id, seq"#,
        )
        .map_err(err)?;
    type FkRow = (i64, String, String, Option<String>, String, String);
    let rows: Vec<FkRow> = stmt
        .query_map(params![table, schema], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)?;

    let mut keys: Vec<(i64, ForeignKey, bool)> = Vec::new(); // id, key, refers to the parent's PK
    for (id, ref_table, from, to, on_update, on_delete) in rows {
        if keys.last().is_none_or(|(last, ..)| *last != id) {
            keys.push((
                id,
                ForeignKey {
                    // SQLite doesn't keep constraint names where PRAGMAs can read them.
                    name: format!("{table}_fk{id}"),
                    columns: Vec::new(),
                    // A foreign key always points into the same database.
                    ref_schema: schema.to_owned(),
                    ref_table,
                    ref_columns: Vec::new(),
                    on_delete: fk_action(&on_delete),
                    on_update: fk_action(&on_update),
                },
                to.is_none(),
            ));
        }
        let (_, key, _) = keys.last_mut().expect("pushed above");
        key.columns.push(from);
        if let Some(to) = to {
            key.ref_columns.push(to);
        }
    }

    // `REFERENCES parent` without columns means the parent's primary key.
    let mut out = Vec::with_capacity(keys.len());
    for (_, mut key, to_pk) in keys {
        if to_pk {
            key.ref_columns = primary_key(&column_rows(conn, schema, &key.ref_table)?);
        }
        out.push(key);
    }
    Ok(out)
}

fn fk_action(action: &str) -> FkAction {
    match action {
        "RESTRICT" => FkAction::Restrict,
        "CASCADE" => FkAction::Cascade,
        "SET NULL" => FkAction::SetNull,
        "SET DEFAULT" => FkAction::SetDefault,
        _ => FkAction::NoAction,
    }
}

fn indexes(conn: &Connection, schema: &str, table: &str) -> DbResult<Vec<Index>> {
    let mut stmt = conn
        .prepare(r#"SELECT name, "unique", origin FROM pragma_index_list(?1, ?2) ORDER BY name"#)
        .map_err(err)?;
    let list: Vec<(String, bool, String)> = stmt
        .query_map(params![table, schema], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)?;

    let sql_of = format!(
        "SELECT sql FROM {}.sqlite_schema WHERE type = 'index' AND name = ?1",
        SqliteDialect.quote_ident(schema)
    );
    let mut out = Vec::with_capacity(list.len());
    for (name, unique, origin) in list {
        let mut cols = conn
            .prepare("SELECT name FROM pragma_index_info(?1, ?2) ORDER BY seqno")
            .map_err(err)?;
        // Expression parts have no name and are left out (see `definition`).
        let columns: Vec<String> = cols
            .query_map(params![name, schema], |row| row.get::<_, Option<String>>(0))
            .map_err(err)?
            .filter_map(|r| r.transpose())
            .collect::<Result<_, _>>()
            .map_err(err)?;
        let sql: Option<String> = conn
            .query_row(&sql_of, [&name], |row| row.get(0))
            .unwrap_or(None);
        // Indexes made for UNIQUE / PRIMARY KEY constraints have no SQL.
        let definition = sql.unwrap_or_else(|| {
            let d = SqliteDialect;
            let cols: Vec<String> = columns.iter().map(|c| d.quote_ident(c)).collect();
            format!(
                "-- automatic index for a {} constraint\nCREATE UNIQUE INDEX {} ON {} ({})",
                if origin == "pk" {
                    "PRIMARY KEY"
                } else {
                    "UNIQUE"
                },
                d.quote_ident(&name),
                d.quote_ident(table),
                cols.join(", ")
            )
        });
        out.push(Index {
            name,
            columns,
            unique,
            primary: origin == "pk",
            definition,
        });
    }
    Ok(out)
}
