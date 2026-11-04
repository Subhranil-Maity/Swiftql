use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "scope", rename_all = "camelCase")]
pub enum IntrospectScope {
    Schemas,                                        // "What schemas exist?"
    Tables { schema: String },                      // "What tables are in this schema?"
    TableDetails { schema: String, table: String }, // "Tell me everything about this table"
    Catalog, // every user schema with its tables and columns, in one go (autocomplete)
    /// Like Catalog, plus keys, indexes, comments and sizes (the Tables and Graph views).
    Model,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", content = "data", rename_all = "camelCase")]
pub enum SchemaSnapshot {
    Schemas(Vec<Schema>), // you asked: "what schemas exist?"
    Tables(Vec<Table>),   // you asked: "what tables are in this schema?"
    TableDetails(Table),  // you asked: "give me everything about this table"
    #[serde(rename_all = "camelCase")]
    Catalog {
        schemas: Vec<Schema>, // tables and columns filled in
        /// Schemas unqualified names resolve against, in order (existing ones only).
        search_path: Vec<String>,
    },
    /// Same shape as Catalog; the optional table and column fields are filled
    /// in where the driver knows them.
    #[serde(rename_all = "camelCase")]
    Model {
        schemas: Vec<Schema>,
        search_path: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    pub name: String,
    pub tables: Option<Vec<Table>>, // None = not loaded yet
}

/// The `Option` fields beyond `columns` are only filled by `IntrospectScope::Model`
/// (None = not loaded, or the driver doesn't know).
#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub name: String,
    pub kind: TableKind,
    pub columns: Option<Vec<Column>>, // None = not loaded yet
    pub comment: Option<String>,
    /// The planner's estimate, not a count.
    pub row_estimate: Option<i64>,
    /// Table, indexes and TOAST together.
    pub size_bytes: Option<i64>,
    /// Column names in key order; Some(empty) = loaded, no primary key.
    pub primary_key: Option<Vec<String>>,
    pub foreign_keys: Option<Vec<ForeignKey>>,
    pub indexes: Option<Vec<Index>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TableKind {
    #[default]
    Table,
    View,
    MaterializedView, // PG only
    ForeignTable,     // PG foreign data wrapper
    PartitionedTable, // PG 10+ partition parent
}

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub name: String,
    pub data_type: String, // "integer", "text", ...
    pub nullable: bool,
    pub comment: Option<String>,
    /// The default expression as SQL, e.g. "nextval('users_id_seq'::regclass)".
    pub default: Option<String>,
}

/// `columns[i]` references `ref_columns[i]`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_schema: String,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
    pub on_delete: FkAction,
    pub on_update: FkAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum FkAction {
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Index {
    pub name: String,
    /// Column names in index order; expression parts are left out (see `definition`).
    pub columns: Vec<String>,
    pub unique: bool,
    pub primary: bool,
    /// Full `CREATE INDEX` statement as the server prints it.
    pub definition: String,
}
