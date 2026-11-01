use std::error::Error;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};
use driver_core::{
    types::{ColumnMeta, LogicalType},
    value::Value,
};
use fallible_iterator::FallibleIterator;
use postgres_protocol::types::{self as pg, Range, RangeBound};
use tokio_postgres::{
    Column, Row,
    types::{FromSql, Kind, Type},
};

type DecodeResult<T> = Result<T, Box<dyn Error + Sync + Send>>;

/// Largest integer JavaScript can hold exactly (2^53 - 1).
const MAX_SAFE_INT: i64 = (1 << 53) - 1;

/// Built-in types decoded by this driver. Everything else becomes Value::Unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Builtin {
    Bool,
    Int2,
    Int4,
    Int8,
    Oid,
    Float4,
    Float8,
    Numeric,
    Text,
    Bytea,
    Uuid,
    Json,
    Jsonb,
    Date,
    Time,
    TimeTz,
    Timestamp,
    TimestampTz,
    Interval,
    Inet,
    Cidr,
    Money,
}

fn builtin(ty: &Type) -> Option<Builtin> {
    use Builtin::*;
    if ty.name() == "citext" {
        return Some(Text); // extension type, text on the wire
    }
    if ty.schema() != "pg_catalog" {
        return None;
    }
    Some(match ty.name() {
        "bool" => Bool,
        "int2" => Int2,
        "int4" => Int4,
        "int8" => Int8,
        "oid" => Oid,
        "float4" => Float4,
        "float8" => Float8,
        "numeric" => Numeric,
        "text" | "varchar" | "bpchar" | "name" | "char" => Text,
        "bytea" => Bytea,
        "uuid" => Uuid,
        "json" => Json,
        "jsonb" => Jsonb,
        "date" => Date,
        "time" => Time,
        "timetz" => TimeTz,
        "timestamp" => Timestamp,
        "timestamptz" => TimestampTz,
        "interval" => Interval,
        "inet" => Inet,
        "cidr" => Cidr,
        "money" => Money,
        _ => return None,
    })
}

/// Postgres type → LogicalType (unmapped → Other).
pub fn logical_type(ty: &Type) -> LogicalType {
    match ty.kind() {
        Kind::Enum(_) => return LogicalType::Enum,
        Kind::Array(elem) => return LogicalType::Array(Box::new(logical_type(elem))),
        Kind::Range(elem) => return LogicalType::Range(Box::new(logical_type(elem))),
        Kind::Domain(base) => return logical_type(base),
        _ => {}
    }
    match builtin(ty) {
        Some(Builtin::Bool) => LogicalType::Bool,
        Some(Builtin::Int2 | Builtin::Int4 | Builtin::Int8 | Builtin::Oid) => LogicalType::Int,
        Some(Builtin::Float4 | Builtin::Float8) => LogicalType::Float,
        Some(Builtin::Numeric) => LogicalType::Decimal,
        Some(Builtin::Text) => LogicalType::Text,
        Some(Builtin::Bytea) => LogicalType::Binary,
        Some(Builtin::Uuid) => LogicalType::Uuid,
        Some(Builtin::Json | Builtin::Jsonb) => LogicalType::Json,
        Some(Builtin::Date) => LogicalType::Date,
        Some(Builtin::Time) => LogicalType::Time,
        Some(Builtin::TimeTz) => LogicalType::TimeTz,
        Some(Builtin::Timestamp) => LogicalType::Timestamp,
        Some(Builtin::TimestampTz) => LogicalType::TimestampTz,
        Some(Builtin::Interval) => LogicalType::Interval,
        Some(Builtin::Inet) => LogicalType::Inet,
        Some(Builtin::Cidr) => LogicalType::Cidr,
        Some(Builtin::Money) => LogicalType::Money,
        None => LogicalType::Other,
    }
}

pub fn column_meta(column: &Column) -> ColumnMeta {
    ColumnMeta {
        name: column.name().to_owned(),
        logical: logical_type(column.type_()),
        // TODO: typmod ("numeric(12,2)") isn't exposed by tokio-postgres
        native_type: column.type_().name().to_owned(),
        nullable: None,     // TODO: look up pg_attribute via table_oid / column_id
        source_table: None, // TODO: needed for editable_results
    }
}

/// Decode one row into Values, in column order.
pub fn decode_row(row: &Row) -> Vec<Value> {
    row.columns()
        .iter()
        .enumerate()
        .map(|(i, column)| match row.try_get::<_, Raw>(i) {
            Ok(Raw(Some(raw))) => decode(column.type_(), raw),
            Ok(Raw(None)) => Value::Null,
            Err(err) => Value::Unknown {
                type_name: column.type_().name().to_owned(),
                text: err.to_string(),
            },
        })
        .collect()
}

/// Raw column bytes, so every type is decoded here instead of through FromSql impls.
struct Raw<'a>(Option<&'a [u8]>);

impl<'a> FromSql<'a> for Raw<'a> {
    fn from_sql(_: &Type, raw: &'a [u8]) -> DecodeResult<Self> {
        Ok(Raw(Some(raw)))
    }

    fn from_sql_null(_: &Type) -> DecodeResult<Self> {
        Ok(Raw(None))
    }

    fn accepts(_: &Type) -> bool {
        true
    }
}

/// Binary value → Value. Never fails: anything undecodable becomes Value::Unknown.
fn decode(ty: &Type, raw: &[u8]) -> Value {
    try_decode(ty, raw).unwrap_or_else(|_| unknown(ty, raw))
}

fn decode_opt(ty: &Type, raw: Option<&[u8]>) -> Value {
    raw.map_or(Value::Null, |raw| decode(ty, raw))
}

fn try_decode(ty: &Type, raw: &[u8]) -> DecodeResult<Value> {
    match ty.kind() {
        Kind::Enum(_) => return Ok(Value::Text(pg::text_from_sql(raw)?.to_owned())),
        Kind::Array(elem) => return decode_array(elem, raw),
        Kind::Range(elem) => return decode_range(elem, raw),
        Kind::Domain(base) => return try_decode(base, raw),
        // TODO: composite, multirange and pseudo types (e.g. ROW(1, 2)) fall back to Unknown
        _ => {}
    }
    let Some(builtin) = builtin(ty) else {
        return Ok(unknown(ty, raw));
    };

    Ok(match builtin {
        Builtin::Bool => Value::Bool(pg::bool_from_sql(raw)?),
        Builtin::Int2 => Value::Int(pg::int2_from_sql(raw)?.into()),
        Builtin::Int4 => Value::Int(pg::int4_from_sql(raw)?.into()),
        Builtin::Int8 => {
            let n = pg::int8_from_sql(raw)?;
            if (-MAX_SAFE_INT..=MAX_SAFE_INT).contains(&n) {
                Value::Int(n)
            } else {
                Value::BigInt(n.to_string())
            }
        }
        Builtin::Oid => Value::Int(pg::oid_from_sql(raw)?.into()),
        Builtin::Float4 => float(ty, pg::float4_from_sql(raw)?.into()),
        Builtin::Float8 => float(ty, pg::float8_from_sql(raw)?),
        Builtin::Numeric => Value::Decimal(decode_numeric(raw)?),
        Builtin::Text => Value::Text(pg::text_from_sql(raw)?.to_owned()),
        Builtin::Bytea => Value::Bytes(pg::bytea_from_sql(raw).to_vec()),
        Builtin::Uuid => Value::Uuid(uuid::Uuid::from_bytes(pg::uuid_from_sql(raw)?)),
        Builtin::Json => Value::Json(serde_json::from_slice(raw)?),
        Builtin::Jsonb => {
            let (version, json) = raw.split_first().ok_or("empty jsonb value")?;
            if *version != 1 {
                return Err(format!("unsupported jsonb version {version}").into());
            }
            Value::Json(serde_json::from_slice(json)?)
        }
        Builtin::Date => match pg::date_from_sql(raw)? {
            i32::MAX => special(ty, "infinity"),
            i32::MIN => special(ty, "-infinity"),
            days => Value::Date(
                pg_epoch()
                    .date()
                    .checked_add_signed(TimeDelta::days(days.into()))
                    .ok_or("date out of range")?,
            ),
        },
        Builtin::Time => Value::Time(time_from_micros(pg::time_from_sql(raw)?)?),
        Builtin::TimeTz => {
            let raw: &[u8; 12] = raw.try_into().map_err(|_| "invalid timetz length")?;
            let micros = i64::from_be_bytes(raw[..8].try_into()?);
            let zone = i32::from_be_bytes(raw[8..].try_into()?);
            Value::TimeTz {
                time: time_from_micros(micros)?,
                offset_seconds: -zone, // Postgres sends seconds west of UTC
            }
        }
        Builtin::Timestamp => match pg::timestamp_from_sql(raw)? {
            i64::MAX => special(ty, "infinity"),
            i64::MIN => special(ty, "-infinity"),
            micros => Value::Timestamp(timestamp_from_micros(micros)?),
        },
        Builtin::TimestampTz => match pg::timestamp_from_sql(raw)? {
            i64::MAX => special(ty, "infinity"),
            i64::MIN => special(ty, "-infinity"),
            micros => Value::TimestampTz(timestamp_from_micros(micros)?.and_utc()),
        },
        Builtin::Interval => {
            let raw: &[u8; 16] = raw.try_into().map_err(|_| "invalid interval length")?;
            Value::Interval {
                microseconds: i64::from_be_bytes(raw[..8].try_into()?),
                days: i32::from_be_bytes(raw[8..12].try_into()?),
                months: i32::from_be_bytes(raw[12..].try_into()?),
            }
        }
        Builtin::Inet => {
            let inet = pg::inet_from_sql(raw)?;
            Value::Inet {
                addr: inet.addr(),
                prefix: inet.netmask(),
            }
        }
        Builtin::Cidr => {
            let inet = pg::inet_from_sql(raw)?;
            Value::Cidr {
                addr: inet.addr(),
                prefix: inet.netmask(),
            }
        }
        // TODO: fractional digits depend on lc_monetary; assumes 2 (most locales)
        Builtin::Money => Value::Money(format_money(pg::int8_from_sql(raw)?)),
    })
}

fn decode_array(elem: &Type, raw: &[u8]) -> DecodeResult<Value> {
    let array = pg::array_from_sql(raw)?;
    let dims: Vec<usize> = array
        .dimensions()
        .map(|d| Ok(usize::try_from(d.len)?))
        .collect()?;
    let values: Vec<Value> = array.values().map(|v| Ok(decode_opt(elem, v))).collect()?;
    if dims.is_empty() {
        return Ok(Value::Array(Vec::new())); // '{}'
    }
    Ok(nest(&dims, &mut values.into_iter()))
}

/// Flat row-major values → nested arrays, one level per dimension.
fn nest(dims: &[usize], values: &mut impl Iterator<Item = Value>) -> Value {
    match dims {
        [] => Value::Array(Vec::new()),
        [len] => Value::Array(values.take(*len).collect()),
        [len, rest @ ..] => Value::Array((0..*len).map(|_| nest(rest, values)).collect()),
    }
}

fn decode_range(elem: &Type, raw: &[u8]) -> DecodeResult<Value> {
    Ok(match pg::range_from_sql(raw)? {
        Range::Empty => Value::Range {
            lower: None,
            upper: None,
            lower_inclusive: false,
            upper_inclusive: false,
            empty: true,
        },
        Range::Nonempty(lower, upper) => {
            let (lower, lower_inclusive) = range_bound(elem, lower);
            let (upper, upper_inclusive) = range_bound(elem, upper);
            Value::Range {
                lower,
                upper,
                lower_inclusive,
                upper_inclusive,
                empty: false,
            }
        }
    })
}

fn range_bound(elem: &Type, bound: RangeBound<Option<&[u8]>>) -> (Option<Box<Value>>, bool) {
    match bound {
        RangeBound::Inclusive(v) => (Some(Box::new(decode_opt(elem, v))), true),
        RangeBound::Exclusive(v) => (Some(Box::new(decode_opt(elem, v))), false),
        RangeBound::Unbounded => (None, false),
    }
}

/// Binary numeric (base-10000 digits) → decimal string, keeping the scale.
fn decode_numeric(raw: &[u8]) -> DecodeResult<String> {
    let word = |i: usize| -> DecodeResult<u16> {
        let bytes = raw.get(i..i + 2).ok_or("numeric value too short")?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    };
    let ndigits = usize::from(word(0)?);
    let weight = i32::from(word(2)? as i16); // position of the first digit, in groups
    let sign = word(4)?;
    let dscale = usize::from(word(6)?); // digits after the decimal point
    if raw.len() != 8 + ndigits * 2 {
        return Err("invalid numeric length".into());
    }
    let digit = |group: i32| -> DecodeResult<u16> {
        match usize::try_from(group) {
            Ok(g) if g < ndigits => word(8 + g * 2),
            _ => Ok(0),
        }
    };

    let mut out = match sign {
        0x0000 => String::new(),
        0x4000 => String::from("-"),
        0xC000 => return Ok("NaN".into()),
        0xD000 => return Ok("Infinity".into()),
        0xF000 => return Ok("-Infinity".into()),
        _ => return Err("invalid numeric sign".into()),
    };

    if weight < 0 {
        out.push('0');
    } else {
        out.push_str(&digit(0)?.to_string());
        for group in 1..=weight {
            out.push_str(&format!("{:04}", digit(group)?));
        }
    }

    if dscale > 0 {
        out.push('.');
        let start = out.len();
        let mut group = weight + 1;
        while out.len() - start < dscale {
            out.push_str(&format!("{:04}", digit(group)?));
            group += 1;
        }
        out.truncate(start + dscale);
    }
    Ok(out)
}

fn float(ty: &Type, v: f64) -> Value {
    // JSON has no NaN / Infinity (serde_json would send null)
    if v.is_nan() {
        special(ty, "NaN")
    } else if v.is_infinite() {
        special(ty, if v > 0.0 { "Infinity" } else { "-Infinity" })
    } else {
        Value::Float(v)
    }
}

fn format_money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

/// 2000-01-01 00:00:00, the Postgres epoch.
fn pg_epoch() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2000, 1, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .expect("valid date")
}

fn timestamp_from_micros(micros: i64) -> DecodeResult<NaiveDateTime> {
    Ok(pg_epoch()
        .checked_add_signed(TimeDelta::microseconds(micros))
        .ok_or("timestamp out of range")?)
}

/// Microseconds since midnight → NaiveTime. 24:00:00 can't be represented and errors.
fn time_from_micros(micros: i64) -> DecodeResult<NaiveTime> {
    let secs = u32::try_from(micros.div_euclid(1_000_000))?;
    let nanos = u32::try_from(micros.rem_euclid(1_000_000) * 1000)?;
    Ok(NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos).ok_or("time out of range")?)
}

/// Values like 'infinity' that the logical type can't hold.
fn special(ty: &Type, text: &str) -> Value {
    Value::Unknown {
        type_name: ty.name().to_owned(),
        text: text.to_owned(),
    }
}

/// Fallback: readable text if the bytes look like text, otherwise \x hex (bytea style).
fn unknown(ty: &Type, raw: &[u8]) -> Value {
    let text = match std::str::from_utf8(raw) {
        Ok(s)
            if !s
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')) =>
        {
            s.to_owned()
        }
        _ => {
            let hex: String = raw.iter().map(|b| format!("{b:02x}")).collect();
            format!("\\x{hex}")
        }
    };
    Value::Unknown {
        type_name: ty.name().to_owned(),
        text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numeric(ndigits: u16, weight: i16, sign: u16, dscale: u16, digits: &[u16]) -> Vec<u8> {
        let mut buf = Vec::new();
        for w in [ndigits, weight as u16, sign, dscale].iter().chain(digits) {
            buf.extend_from_slice(&w.to_be_bytes());
        }
        buf
    }

    #[test]
    fn numeric_values() {
        // 12345.678 → groups [1, 2345, 6780], weight 1
        let raw = numeric(3, 1, 0, 3, &[1, 2345, 6780]);
        assert_eq!(decode_numeric(&raw).unwrap(), "12345.678");
        // -0.0001 → groups [1], weight -1
        let raw = numeric(1, -1, 0x4000, 4, &[1]);
        assert_eq!(decode_numeric(&raw).unwrap(), "-0.0001");
        // 0.00000001 → groups [1], weight -2
        let raw = numeric(1, -2, 0, 8, &[1]);
        assert_eq!(decode_numeric(&raw).unwrap(), "0.00000001");
        // 10000.00 → groups [1], weight 1 (trailing zero groups are dropped)
        let raw = numeric(1, 1, 0, 2, &[1]);
        assert_eq!(decode_numeric(&raw).unwrap(), "10000.00");
        assert_eq!(decode_numeric(&numeric(0, 0, 0, 0, &[])).unwrap(), "0");
        assert_eq!(
            decode_numeric(&numeric(0, 0, 0xC000, 0, &[])).unwrap(),
            "NaN"
        );
    }

    #[test]
    fn int8_safe_and_unsafe() {
        let small = decode(&Type::INT8, &42i64.to_be_bytes());
        assert!(matches!(small, Value::Int(42)));
        let big = decode(&Type::INT8, &i64::MAX.to_be_bytes());
        assert!(matches!(big, Value::BigInt(ref s) if s == "9223372036854775807"));
    }

    #[test]
    fn timestamps_and_specials() {
        let one_day = 86_400_000_000i64;
        let ts = decode(&Type::TIMESTAMP, &one_day.to_be_bytes());
        assert!(matches!(ts, Value::Timestamp(t) if t.to_string() == "2000-01-02 00:00:00"));
        let inf = decode(&Type::TIMESTAMPTZ, &i64::MAX.to_be_bytes());
        assert!(matches!(inf, Value::Unknown { ref text, .. } if text == "infinity"));
    }

    #[test]
    fn timetz_offset_is_east_positive() {
        let mut raw = (3_600_000_000i64).to_be_bytes().to_vec(); // 01:00
        raw.extend_from_slice(&(-19_800i32).to_be_bytes()); // +05:30 (sent as west)
        let v = decode(&Type::TIMETZ, &raw);
        assert!(matches!(
            v,
            Value::TimeTz {
                offset_seconds: 19_800,
                ..
            }
        ));
    }

    #[test]
    fn two_dimensional_array() {
        // '{{1,2},{3,4}}'::int4[]
        let mut raw = Vec::new();
        for n in [2i32, 0] {
            raw.extend_from_slice(&n.to_be_bytes()); // ndim, has_nulls
        }
        raw.extend_from_slice(&Type::INT4.oid().to_be_bytes());
        for n in [2i32, 1, 2, 1] {
            raw.extend_from_slice(&n.to_be_bytes()); // (len, lower bound) per dim
        }
        for n in 1i32..=4 {
            raw.extend_from_slice(&4i32.to_be_bytes());
            raw.extend_from_slice(&n.to_be_bytes());
        }
        let Value::Array(rows) = decode(&Type::INT4_ARRAY, &raw) else {
            panic!("not an array");
        };
        assert_eq!(rows.len(), 2);
        assert!(
            matches!(&rows[1], Value::Array(r) if matches!(r[..], [Value::Int(3), Value::Int(4)]))
        );
    }

    #[test]
    fn unknown_falls_back_to_hex() {
        let v = unknown(&Type::POINT, &[0, 1, 255]);
        assert!(matches!(v, Value::Unknown { ref text, .. } if text == "\\x0001ff"));
    }

    #[test]
    fn logical_types() {
        assert_eq!(
            logical_type(&Type::INT4_ARRAY),
            LogicalType::Array(Box::new(LogicalType::Int))
        );
        assert_eq!(
            logical_type(&Type::INT4_RANGE),
            LogicalType::Range(Box::new(LogicalType::Int))
        );
        assert_eq!(logical_type(&Type::JSONB), LogicalType::Json);
        assert_eq!(logical_type(&Type::POINT), LogicalType::Other);
    }
}
