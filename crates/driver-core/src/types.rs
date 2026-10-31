use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMeta {
    pub name: String,
    pub logical: LogicalType, // what the UI uses
    pub native_type: String,  // "numeric(12,2)", "jsonb": for display
    pub nullable: Option<bool>,
    pub source_table: Option<TableRef>, // enables editing results later
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "of", rename_all = "camelCase")]
pub enum LogicalType {
    Bool,
    Int,     // int2, int4, int8, serial, MySQL TINYINT..BIGINT
    Float,   // float4, float8, MySQL FLOAT/DOUBLE
    Decimal, // numeric, MySQL DECIMAL (kept as string for precision)
    Text,    // text, varchar, char, citext
    Binary,  // bytea, MySQL BLOB
    Uuid,
    Json, // json, jsonb
    Date,
    Time,
    TimeTz,                  // Postgres timetz
    Timestamp,               // without time zone
    TimestampTz,             // with time zone
    Interval,                // Postgres interval
    Inet,                    // Postgres inet
    Cidr,                    // Postgres cidr
    Money,                   // Postgres money (formatted text)
    Range(Box<LogicalType>), // int4range → Range(Int)
    Array(Box<LogicalType>), // int4[] → Array(Int)
    Enum,                    // user-defined enums
    Other,                   // anything unmapped: shown as text, never crashes
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRef {
    pub schema: String, // "public"
    pub table: String,  // "users"
    pub column: String, // "email"  (the real column name in the table)
}
