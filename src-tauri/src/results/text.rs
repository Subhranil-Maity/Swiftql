//! A value as Postgres text, close to what psql prints. Mirrors
//! `valueText` in `src/features/console/model/cell-format.ts`, which formats
//! the cells the grid shows; exports and copies use this one.

use driver_core::value::Value;

pub fn value_text(value: &Value) -> String {
    match value {
        Value::Null => "NULL".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => float_text(*f),
        Value::Decimal(s) | Value::BigInt(s) | Value::Text(s) | Value::Money(s) => s.clone(),
        Value::Uuid(u) => u.to_string(),
        Value::Date(d) => d.format("%Y-%m-%d").to_string(),
        Value::Time(t) => t.format("%H:%M:%S%.f").to_string(),
        Value::Bytes(bytes) => {
            let mut s = String::with_capacity(2 + bytes.len() * 2);
            s.push_str("\\x");
            for b in bytes {
                s.push_str(&format!("{b:02x}"));
            }
            s
        }
        Value::Json(j) => j.to_string(),
        Value::TimeTz {
            time,
            offset_seconds,
        } => format!(
            "{}{}",
            time.format("%H:%M:%S%.f"),
            offset_text(*offset_seconds)
        ),
        Value::Timestamp(ts) => ts.format("%Y-%m-%d %H:%M:%S%.f").to_string(),
        // Shown in UTC; the server would use the session's TimeZone.
        Value::TimestampTz(ts) => format!("{}+00", ts.format("%Y-%m-%d %H:%M:%S%.f")),
        Value::Interval {
            months,
            days,
            microseconds,
        } => interval_text(*months, *days, *microseconds),
        Value::Inet { addr, prefix } => {
            let max = if addr.is_ipv6() { 128 } else { 32 };
            if *prefix == max {
                addr.to_string()
            } else {
                format!("{addr}/{prefix}")
            }
        }
        Value::Cidr { addr, prefix } => format!("{addr}/{prefix}"),
        Value::Range {
            lower,
            upper,
            lower_inclusive,
            upper_inclusive,
            empty,
        } => {
            if *empty {
                return "empty".into();
            }
            let bound = |v: &Option<Box<Value>>| {
                v.as_deref()
                    .map(|v| {
                        quote_element(&value_text(v), |c| {
                            c.is_whitespace() || ",()[]\"\\".contains(c)
                        })
                    })
                    .unwrap_or_default()
            };
            format!(
                "{}{},{}{}",
                if *lower_inclusive { '[' } else { '(' },
                bound(lower),
                bound(upper),
                if *upper_inclusive { ']' } else { ')' }
            )
        }
        Value::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|v| match v {
                    Value::Null => "NULL".into(),
                    Value::Array(_) => value_text(v),
                    _ => quote_element(&value_text(v), |c| {
                        c.is_whitespace() || ",{}\"\\".contains(c)
                    }),
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Unknown { text, .. } => text.clone(),
    }
}

/// Postgres spelling of special floats.
pub fn float_text(f: f64) -> String {
    if f.is_nan() {
        "NaN".into()
    } else if f.is_infinite() {
        if f > 0.0 { "Infinity" } else { "-Infinity" }.into()
    } else {
        f.to_string()
    }
}

/// "+05:30", "-08", "+00" like Postgres prints UTC offsets.
fn offset_text(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let abs = seconds.unsigned_abs();
    let (h, m, s) = (abs / 3600, (abs % 3600) / 60, abs % 60);
    let mut out = format!("{sign}{h:02}");
    if m != 0 || s != 0 {
        out.push_str(&format!(":{m:02}"));
    }
    if s != 0 {
        out.push_str(&format!(":{s:02}"));
    }
    out
}

/// IntervalStyle = postgres: "1 year 2 mons 3 days 04:05:06.5".
fn interval_text(months: i32, days: i32, microseconds: i64) -> String {
    let mut parts = Vec::new();
    let unit =
        |n: i64, one: &str, many: &str| format!("{n} {}", if n.abs() == 1 { one } else { many });
    let (years, mons) = (months / 12, months % 12);
    if years != 0 {
        parts.push(unit(years.into(), "year", "years"));
    }
    if mons != 0 {
        parts.push(unit(mons.into(), "mon", "mons"));
    }
    if days != 0 {
        parts.push(unit(days.into(), "day", "days"));
    }
    if microseconds != 0 || parts.is_empty() {
        let sign = if microseconds < 0 { "-" } else { "" };
        let abs = microseconds.unsigned_abs();
        let total = abs / 1_000_000;
        let fraction = abs % 1_000_000;
        let mut time = format!(
            "{sign}{:02}:{:02}:{:02}",
            total / 3600,
            (total / 60) % 60,
            total % 60
        );
        if fraction != 0 {
            let digits = format!("{fraction:06}");
            time.push('.');
            time.push_str(digits.trim_end_matches('0'));
        }
        parts.push(time);
    }
    parts.join(" ")
}

/// Array and range elements are quoted when they'd be ambiguous unquoted.
fn quote_element(text: &str, special: impl Fn(char) -> bool) -> String {
    if text.is_empty() || text.chars().any(special) || text.eq_ignore_ascii_case("NULL") {
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specials() {
        assert_eq!(value_text(&Value::Float(f64::NAN)), "NaN");
        assert_eq!(value_text(&Value::Float(f64::NEG_INFINITY)), "-Infinity");
        assert_eq!(value_text(&Value::Float(1.5)), "1.5");
        assert_eq!(value_text(&Value::Bytes(vec![0xde, 0xad])), "\\xdead");
    }

    #[test]
    fn intervals_and_offsets() {
        assert_eq!(
            interval_text(14, 3, 14_706_500_000),
            "1 year 2 mons 3 days 04:05:06.5"
        );
        assert_eq!(interval_text(0, 0, 0), "00:00:00");
        assert_eq!(offset_text(19_800), "+05:30");
        assert_eq!(offset_text(-28_800), "-08");
    }

    #[test]
    fn arrays_quote_ambiguous_elements() {
        let v = Value::Array(vec![
            Value::Text("a b".into()),
            Value::Null,
            Value::Text("null".into()),
            Value::Int(1),
        ]);
        assert_eq!(value_text(&v), r#"{"a b",NULL,"null",1}"#);
    }
}
