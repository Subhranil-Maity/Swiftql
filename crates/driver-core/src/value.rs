use std::net::IpAddr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
// TS name is DbValue so it does not clash with serde_json::Value (container
// rename does not change the JSON).
#[serde(
    rename = "DbValue",
    tag = "type",
    content = "value",
    rename_all = "camelCase"
)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(
        #[serde(with = "float_repr")]
        #[cfg_attr(feature = "specta", specta(type = FloatRepr))]
        f64,
    ),
    Decimal(String), // never f64; preserves precision
    BigInt(String),  // anything larger than JS can hold safely
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
    Inet {
        addr: IpAddr,
        prefix: u8,
    }, // host address, prefix may be < max
    Cidr {
        addr: IpAddr,
        prefix: u8,
    }, // network address
    Money(String), // formatted text; locale dependent
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
    Unknown {
        type_name: String,
        text: String,
    }, // fallback for anything unmapped
}

/// How a float travels to the UI: JSON has no NaN or infinities, so those
/// are sent as the strings Postgres prints.
#[cfg(feature = "specta")]
#[derive(Serialize, specta::Type)]
#[allow(dead_code)]
#[serde(rename = "FloatValue", untagged)]
enum FloatRepr {
    Number(f64),
    Special(SpecialFloat),
}

#[cfg(feature = "specta")]
#[derive(Serialize, specta::Type)]
#[allow(dead_code)]
enum SpecialFloat {
    #[serde(rename = "NaN")]
    NaN,
    #[serde(rename = "Infinity")]
    Infinity,
    #[serde(rename = "-Infinity")]
    NegInfinity,
}

mod float_repr {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
        if v.is_nan() {
            s.serialize_str("NaN")
        } else if v.is_infinite() {
            s.serialize_str(if *v > 0.0 { "Infinity" } else { "-Infinity" })
        } else {
            s.serialize_f64(*v)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Number(f64),
            Text(String),
        }
        match Repr::deserialize(d)? {
            Repr::Number(n) => Ok(n),
            Repr::Text(t) => match t.as_str() {
                "NaN" => Ok(f64::NAN),
                "Infinity" => Ok(f64::INFINITY),
                "-Infinity" => Ok(f64::NEG_INFINITY),
                other => Err(D::Error::custom(format!("invalid float {other:?}"))),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn special_floats_round_trip_as_text() {
        let json = serde_json::to_string(&Value::Float(f64::NAN)).unwrap();
        assert_eq!(json, r#"{"type":"float","value":"NaN"}"#);
        let json = serde_json::to_string(&Value::Float(f64::NEG_INFINITY)).unwrap();
        assert_eq!(json, r#"{"type":"float","value":"-Infinity"}"#);
        assert_eq!(
            serde_json::to_string(&Value::Float(1.5)).unwrap(),
            r#"{"type":"float","value":1.5}"#
        );
        let back: Value = serde_json::from_str(r#"{"type":"float","value":"Infinity"}"#).unwrap();
        assert!(matches!(back, Value::Float(f) if f == f64::INFINITY));
    }
}
