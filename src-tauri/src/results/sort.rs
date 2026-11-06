//! Sorting a held result: type-aware, stable, NULLs always last whichever the
//! direction. Ported from the grid's former client-side sort.

use std::cmp::Ordering;

use driver_core::value::Value;
use serde::Deserialize;
use specta::Type;

use super::text::value_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Row indices in sorted order; ties keep their original order.
pub fn sort_order(rows: &[Vec<Value>], column: usize, direction: SortDirection) -> Vec<u32> {
    let mut order: Vec<u32> = (0..rows.len() as u32).collect();
    let cell = |i: u32| {
        rows[i as usize]
            .get(column)
            .filter(|v| !matches!(v, Value::Null))
    };
    // sort_by is stable, so equal rows keep their order.
    order.sort_by(|&i, &j| match (cell(i), cell(j)) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => {
            let ord = compare_values(a, b);
            if direction == SortDirection::Desc {
                ord.reverse()
            } else {
                ord
            }
        }
    });
    order
}

/// Two non-NULL values of the same column.
pub fn compare_values(a: &Value, b: &Value) -> Ordering {
    use Value::*;
    match (a, b) {
        (Int(x), Int(y)) => x.cmp(y),
        (Float(x), Float(y)) => compare_floats(*x, *y),
        (Bool(x), Bool(y)) => x.cmp(y),
        (Interval { .. }, Interval { .. }) => interval_key(a).cmp(&interval_key(b)),
        (Date(x), Date(y)) => x.cmp(y),
        (Time(x), Time(y)) => x.cmp(y),
        (Timestamp(x), Timestamp(y)) => x.cmp(y),
        (TimestampTz(x), TimestampTz(y)) => x.cmp(y),
        (Uuid(x), Uuid(y)) => x.cmp(y),
        _ if is_numeric(a) && is_numeric(b) => {
            let (ta, tb) = (numeric_text(a), numeric_text(b));
            // Floats can print in exponent form, which the exact compare can't read.
            if has_exponent(&ta) || has_exponent(&tb) {
                compare_floats(
                    ta.parse().unwrap_or(f64::NAN),
                    tb.parse().unwrap_or(f64::NAN),
                )
            } else {
                compare_decimal(&ta, &tb)
            }
        }
        _ => natural_cmp(&value_text(a), &value_text(b)),
    }
}

/// NaN sorts above every number, as on the server.
fn compare_floats(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
    }
}

fn has_exponent(s: &str) -> bool {
    s.contains(['e', 'E']) && !s.contains("NaN")
}

fn is_numeric(v: &Value) -> bool {
    matches!(
        v,
        Value::Int(_) | Value::Float(_) | Value::Decimal(_) | Value::BigInt(_)
    )
}

fn numeric_text(v: &Value) -> String {
    match v {
        Value::Float(f) if f.is_nan() => "NaN".into(),
        Value::Float(f) if f.is_infinite() => {
            if *f > 0.0 {
                "1e999".into()
            } else {
                "-1e999".into()
            }
        }
        other => value_text(other),
    }
}

/// Microseconds, roughly: a month counts as 30 days, as Postgres does when comparing.
fn interval_key(v: &Value) -> i128 {
    match v {
        Value::Interval {
            months,
            days,
            microseconds,
        } => {
            (i128::from(*months) * 30 + i128::from(*days)) * 86_400_000_000
                + i128::from(*microseconds)
        }
        _ => 0,
    }
}

/// Compares decimal strings exactly ("-12.50" vs "3"), with no float rounding.
/// Postgres' numeric NaN sorts above every number, as on the server.
pub fn compare_decimal(a: &str, b: &str) -> Ordering {
    let (nan_a, nan_b) = (a == "NaN", b == "NaN");
    if nan_a || nan_b {
        return nan_a.cmp(&nan_b);
    }
    let (neg_a, neg_b) = (a.starts_with('-'), b.starts_with('-'));
    if neg_a != neg_b {
        return if neg_a {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }
    let split = |s: &str| {
        let s = s.trim_start_matches(['-', '+']);
        let (int, frac) = s.split_once('.').unwrap_or((s, ""));
        let int = int.trim_start_matches('0');
        (int.to_string(), frac.to_string())
    };
    let (ia, fa) = split(a);
    let (ib, fb) = split(b);
    let magnitude = ia
        .len()
        .cmp(&ib.len())
        .then_with(|| ia.cmp(&ib))
        .then_with(|| {
            let width = fa.len().max(fb.len());
            format!("{fa:0<width$}").cmp(&format!("{fb:0<width$}"))
        });
    if neg_a {
        magnitude.reverse()
    } else {
        magnitude
    }
}

/// Case-insensitive, with digit runs compared as numbers ("a2" < "a10").
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return a.cmp(b), // equal ignoring case: fall back to exact
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars<'_>>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (na, nb) = (take(&mut ai), take(&mut bi));
                let ord = compare_decimal(&na, &nb);
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(x), Some(y)) => {
                let ord = x.to_lowercase().cmp(y.to_lowercase());
                if ord != Ordering::Equal {
                    return ord;
                }
                ai.next();
                bi.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_compare_exactly() {
        assert_eq!(compare_decimal("-12.50", "3"), Ordering::Less);
        assert_eq!(compare_decimal("10", "9.99"), Ordering::Greater);
        assert_eq!(compare_decimal("1.10", "1.1"), Ordering::Equal);
        assert_eq!(compare_decimal("-2", "-10"), Ordering::Greater);
        assert_eq!(compare_decimal("NaN", "1e9"), Ordering::Greater);
        assert_eq!(compare_decimal("0.001", "0"), Ordering::Greater);
    }

    #[test]
    fn nulls_last_both_directions_and_stable() {
        let rows = vec![
            vec![Value::Int(2)],
            vec![Value::Null],
            vec![Value::Int(1)],
            vec![Value::Int(2)],
        ];
        assert_eq!(sort_order(&rows, 0, SortDirection::Asc), [2, 0, 3, 1]);
        assert_eq!(sort_order(&rows, 0, SortDirection::Desc), [0, 3, 2, 1]);
    }

    #[test]
    fn floats_with_nan_and_infinity() {
        let rows = vec![
            vec![Value::Float(f64::NAN)],
            vec![Value::Float(f64::INFINITY)],
            vec![Value::Float(-1.5)],
        ];
        assert_eq!(sort_order(&rows, 0, SortDirection::Asc), [2, 1, 0]);
    }

    #[test]
    fn text_is_natural() {
        let rows = vec![
            vec![Value::Text("a10".into())],
            vec![Value::Text("A2".into())],
            vec![Value::Text("b".into())],
        ];
        assert_eq!(sort_order(&rows, 0, SortDirection::Asc), [1, 0, 2]);
    }

    #[test]
    fn mixed_numeric_types() {
        assert_eq!(
            compare_values(&Value::Decimal("2.5".into()), &Value::Int(3)),
            Ordering::Less
        );
        assert_eq!(
            compare_values(&Value::BigInt("9007199254740993".into()), &Value::Int(5)),
            Ordering::Greater
        );
    }
}
