use driver_core::{
    types::{ColumnMeta, LogicalType},
    value::Value,
};
use rusqlite::types::ValueRef;

/// Integers past this go to the UI as text (JS numbers lose precision).
const MAX_SAFE_INT: i64 = (1 << 53) - 1;

/// A column type from its declared type, by SQLite's affinity rules
/// (https://sqlite.org/datatype3.html#determination_of_column_affinity).
/// Values can still be of any storage class; each is decoded on its own.
pub fn logical_type(decl: Option<&str>) -> LogicalType {
    let Some(decl) = decl.filter(|d| !d.trim().is_empty()) else {
        return LogicalType::Other; // an expression: no declared type
    };
    let upper = decl.to_ascii_uppercase();
    if upper.starts_with("BOOL") {
        LogicalType::Bool
    } else if upper.contains("INT") {
        LogicalType::Int
    } else if upper.contains("CHAR") || upper.contains("CLOB") || upper.contains("TEXT") {
        LogicalType::Text
    } else if upper.contains("BLOB") {
        LogicalType::Binary
    } else if upper.contains("REAL") || upper.contains("FLOA") || upper.contains("DOUB") {
        LogicalType::Float
    } else if upper.contains("DATE") || upper.contains("TIME") || upper.contains("JSON") {
        // Stored as text or numbers; shown as stored.
        LogicalType::Text
    } else {
        LogicalType::Decimal // NUMERIC affinity
    }
}

pub fn column_meta(name: &str, decl: Option<&str>) -> ColumnMeta {
    ColumnMeta {
        name: name.to_owned(),
        logical: logical_type(decl),
        native_type: decl.map_or_else(|| "any".to_owned(), str::to_owned),
        nullable: None,
        source_table: None,
    }
}

/// One cell, by its storage class. 0 / 1 in a BOOLEAN column become booleans.
pub fn decode(value: ValueRef<'_>, logical: &LogicalType) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n @ (0 | 1)) if *logical == LogicalType::Bool => Value::Bool(n == 1),
        ValueRef::Integer(n) if (-MAX_SAFE_INT..=MAX_SAFE_INT).contains(&n) => Value::Int(n),
        ValueRef::Integer(n) => Value::BigInt(n.to_string()),
        ValueRef::Real(f) => Value::Float(f),
        ValueRef::Text(bytes) => Value::Text(String::from_utf8_lossy(bytes).into_owned()),
        ValueRef::Blob(bytes) => Value::Bytes(bytes.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affinity() {
        assert_eq!(logical_type(Some("INTEGER")), LogicalType::Int);
        assert_eq!(logical_type(Some("varchar(20)")), LogicalType::Text);
        assert_eq!(logical_type(Some("BLOB")), LogicalType::Binary);
        assert_eq!(logical_type(Some("double precision")), LogicalType::Float);
        assert_eq!(logical_type(Some("NUMERIC(10,2)")), LogicalType::Decimal);
        assert_eq!(logical_type(Some("boolean")), LogicalType::Bool);
        assert_eq!(logical_type(Some("DATETIME")), LogicalType::Text);
        assert_eq!(logical_type(None), LogicalType::Other);
    }

    #[test]
    fn values() {
        let int = LogicalType::Int;
        assert!(matches!(decode(ValueRef::Integer(7), &int), Value::Int(7)));
        assert!(matches!(
            decode(ValueRef::Integer(i64::MAX), &int),
            Value::BigInt(_)
        ));
        assert!(matches!(
            decode(ValueRef::Integer(1), &LogicalType::Bool),
            Value::Bool(true)
        ));
        assert!(matches!(
            decode(ValueRef::Integer(2), &LogicalType::Bool),
            Value::Int(2)
        ));
        assert!(matches!(decode(ValueRef::Text(b"hi"), &int), Value::Text(ref s) if s == "hi"));
    }
}
