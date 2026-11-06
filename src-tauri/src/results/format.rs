//! A held result (or part of one) as CSV, TSV, Markdown or JSON, for Copy as
//! and Export. Rows are written in display (sorted) order.

use std::io::{self, Write};

use driver_core::value::Value;
use serde::Deserialize;
use specta::Type;

use super::{ResultBuffer, ResultSelection, text::value_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Csv,
    Tsv,
    Markdown,
    Json,
}

impl ExportFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Tsv => "TSV",
            Self::Markdown => "Markdown",
            Self::Json => "JSON",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Markdown => "md",
            Self::Json => "json",
        }
    }
}

const NULL: Value = Value::Null;

/// Writes the selected rows and columns; returns how many rows were written.
pub fn write(
    buffer: &ResultBuffer,
    selection: &ResultSelection,
    format: ExportFormat,
    out: &mut impl Write,
) -> io::Result<u32> {
    let columns: Vec<usize> = selection
        .columns
        .iter()
        .map(|&c| c as usize)
        .filter(|&c| c < buffer.columns.len())
        .collect();
    let names: Vec<&str> = columns
        .iter()
        .map(|&c| buffer.columns[c].name.as_str())
        .collect();

    let total = buffer.rows.len() as u32;
    let ranges: Vec<(u32, u32)> = match &selection.rows {
        Some(ranges) => ranges
            .iter()
            .map(|&(s, e)| (s.min(total), e.min(total)))
            .collect(),
        None => vec![(0, total)],
    };
    let positions = ranges.into_iter().flat_map(|(s, e)| s..e);
    let rows = positions
        .filter_map(|pos| buffer.row_at(pos as usize))
        .map(|row| {
            columns
                .iter()
                .map(|&c| row.get(c).unwrap_or(&NULL))
                .collect::<Vec<_>>()
        });

    match format {
        ExportFormat::Csv => delimited(&names, rows, ',', out),
        ExportFormat::Tsv => delimited(&names, rows, '\t', out),
        ExportFormat::Markdown => markdown(&names, rows, out),
        ExportFormat::Json => json(&names, rows, out),
    }
}

fn delimited<'a>(
    names: &[&str],
    rows: impl Iterator<Item = Vec<&'a Value>>,
    sep: char,
    out: &mut impl Write,
) -> io::Result<u32> {
    let quote = |text: &str| -> String {
        let needs = text.contains(sep)
            || text.contains(['"', '\r', '\n'])
            || text.starts_with(char::is_whitespace)
            || text.ends_with(char::is_whitespace);
        if needs {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_string()
        }
    };
    let sep_s = sep.to_string();
    let header: Vec<String> = names.iter().map(|n| quote(n)).collect();
    write!(out, "{}\r\n", header.join(&sep_s))?;
    let mut count = 0;
    for row in rows {
        // NULL is an empty field, the way COPY and spreadsheets expect.
        let fields: Vec<String> = row
            .iter()
            .map(|v| match v {
                Value::Null => String::new(),
                v => quote(&value_text(v)),
            })
            .collect();
        write!(out, "{}\r\n", fields.join(&sep_s))?;
        count += 1;
    }
    Ok(count)
}

fn markdown<'a>(
    names: &[&str],
    rows: impl Iterator<Item = Vec<&'a Value>>,
    out: &mut impl Write,
) -> io::Result<u32> {
    let cell = |text: &str| {
        text.replace('\\', "\\\\")
            .replace('|', "\\|")
            .replace("\r\n", "<br>")
            .replace('\n', "<br>")
    };
    let header: Vec<String> = names.iter().map(|n| cell(n)).collect();
    writeln!(out, "| {} |", header.join(" | "))?;
    writeln!(out, "| {} |", vec!["---"; names.len()].join(" | "))?;
    let mut count = 0;
    for row in rows {
        let cells: Vec<String> = row.iter().map(|v| cell(&value_text(v))).collect();
        writeln!(out, "| {} |", cells.join(" | "))?;
        count += 1;
    }
    Ok(count)
}

/// Numbers that fit a JS number stay numbers; bigint, numeric and special
/// floats stay strings to keep their value.
fn json_value(v: &Value) -> serde_json::Value {
    use serde_json::Value as J;
    match v {
        Value::Null => J::Null,
        Value::Bool(b) => J::Bool(*b),
        Value::Int(i) => J::from(*i),
        Value::Float(f) if f.is_finite() => J::from(*f),
        Value::Json(j) => j.clone(),
        other => J::String(value_text(other)),
    }
}

/// Repeated column names (e.g. two "id"s from a join) get "_2", "_3" so no key is lost.
fn unique_keys(names: &[&str]) -> Vec<String> {
    let mut seen = std::collections::HashMap::new();
    names
        .iter()
        .map(|name| {
            let n = seen.entry(*name).or_insert(0);
            *n += 1;
            if *n == 1 {
                name.to_string()
            } else {
                format!("{name}_{n}")
            }
        })
        .collect()
}

fn json<'a>(
    names: &[&str],
    rows: impl Iterator<Item = Vec<&'a Value>>,
    out: &mut impl Write,
) -> io::Result<u32> {
    let keys = unique_keys(names);
    out.write_all(b"[")?;
    let mut count = 0;
    for row in rows {
        let object: serde_json::Map<String, serde_json::Value> = keys
            .iter()
            .cloned()
            .zip(row.iter().map(|v| json_value(v)))
            .collect();
        out.write_all(if count == 0 { b"\n  " } else { b",\n  " })?;
        serde_json::to_writer(&mut *out, &object)?;
        count += 1;
    }
    out.write_all(if count == 0 { b"]\n" } else { b"\n]\n" })?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use driver_core::types::{ColumnMeta, LogicalType};

    use super::*;
    use crate::results::ResultState;

    fn column(name: &str) -> ColumnMeta {
        ColumnMeta {
            name: name.into(),
            logical: LogicalType::Text,
            native_type: "text".into(),
            nullable: None,
            source_table: None,
        }
    }

    fn buffer() -> ResultBuffer {
        let mut b = ResultBuffer::new(vec![column("id"), column("id"), column("note")]);
        b.push_rows(vec![
            vec![Value::Int(1), Value::Int(2), Value::Text("a,b".into())],
            vec![Value::Int(3), Value::Null, Value::Null],
        ]);
        b.state = ResultState::Complete;
        b
    }

    fn render(format: ExportFormat, rows: Option<Vec<(u32, u32)>>) -> String {
        let mut out = Vec::new();
        let sel = ResultSelection {
            rows,
            columns: vec![0, 1, 2],
        };
        write(&buffer(), &sel, format, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn csv_quotes_and_nulls() {
        assert_eq!(
            render(ExportFormat::Csv, None),
            "id,id,note\r\n1,2,\"a,b\"\r\n3,,\r\n"
        );
    }

    #[test]
    fn json_unique_keys() {
        let text = render(ExportFormat::Json, Some(vec![(1, 2)]));
        assert_eq!(text, "[\n  {\"id\":3,\"id_2\":null,\"note\":null}\n]\n");
    }
}
