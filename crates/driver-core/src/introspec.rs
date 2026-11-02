use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "scope", rename_all = "camelCase")]
pub enum IntrospectScope {
    Schemas,                                        // "What schemas exist?"
    Tables { schema: String },                      // "What tables are in this schema?"
    TableDetails { schema: String, table: String }, // "Tell me everything about this table"
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", content = "data", rename_all = "camelCase")]
pub enum SchemaSnapshot {
    Schemas(Vec<Schema>), // you asked: "what schemas exist?"
    Tables(Vec<Table>),   // you asked: "what tables are in this schema?"
    TableDetails(Table),  // you asked: "give me everything about this table"
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    pub name: String,
    pub tables: Option<Vec<Table>>, // None = not loaded yet
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub name: String,
    pub kind: TableKind,
    pub columns: Option<Vec<Column>>, // None = not loaded yet
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TableKind {
    Table,
    View,
    MaterializedView, // PG only
    ForeignTable,     // PG foreign data wrapper
    PartitionedTable, // PG 10+ partition parent
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub name: String,
    pub data_type: String, // "integer", "text", ...
    pub nullable: bool,
}
