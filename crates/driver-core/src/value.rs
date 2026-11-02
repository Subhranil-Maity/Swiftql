use std::net::IpAddr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
// TS name is DbValue so it does not clash with serde_json::Value (container
// rename does not change the JSON).
#[serde(rename = "DbValue", tag = "type", content = "value", rename_all = "camelCase")]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Decimal(String),          // never f64; preserves precision
    BigInt(String),           // anything larger than JS can hold safely
    Text(String),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    Uuid(uuid::Uuid),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    #[serde(rename_all = "camelCase")]
    TimeTz {
        time: chrono::NaiveTime,
        offset_seconds: i32, // UTC offset, east positive (Postgres timetz)
    },
    Timestamp(chrono::NaiveDateTime),
    TimestampTz(chrono::DateTime<chrono::Utc>),
    // Postgres interval; months/days kept apart since their length varies
    Interval {
        months: i32,
        days: i32,
        microseconds: i64,
    },
    Inet { addr: IpAddr, prefix: u8 }, // host address, prefix may be < max
    Cidr { addr: IpAddr, prefix: u8 }, // network address
    Money(String),                     // formatted text; locale dependent
    #[serde(rename_all = "camelCase")]
    Range {
        lower: Option<Box<Value>>, // None = unbounded
        upper: Option<Box<Value>>, // None = unbounded
        lower_inclusive: bool,
        upper_inclusive: bool,
        empty: bool, // 'empty' range; bounds are ignored
    },
    Array(Vec<Value>),
    #[serde(rename_all = "camelCase")]
    Unknown { type_name: String, text: String }, // fallback for anything unmapped
}
