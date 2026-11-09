//! The command a statement runs ("SELECT", "CREATE TABLE"), read from its
//! leading keywords.

/// Words between CREATE / DROP and the object kind.
const MODIFIERS: &[&str] = &["TEMP", "TEMPORARY", "UNIQUE", "VIRTUAL"];

/// Upper-cased leading words, skipping comments; stops at the first
/// character that isn't part of a word, a comment or whitespace.
fn leading_words(sql: &str, max: usize) -> Vec<String> {
    let mut words = Vec::new();
    let mut rest = sql;
    while words.len() < max {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("--") {
            rest = after.find('\n').map_or("", |i| &after[i + 1..]);
            continue;
        }
        if let Some(after) = rest.strip_prefix("/*") {
            rest = after.find("*/").map_or("", |i| &after[i + 2..]);
            continue;
        }
        let end = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if end == 0 {
            break;
        }
        words.push(rest[..end].to_ascii_uppercase());
        rest = &rest[end..];
    }
    words
}

/// The statement's command. `returns_rows` decides what a WITH or VALUES is.
pub fn command_of(sql: &str, returns_rows: bool) -> String {
    let words = leading_words(sql, 6);
    let Some(first) = words.first() else {
        return String::new();
    };
    match first.as_str() {
        "CREATE" | "DROP" | "ALTER" => {
            let object = words[1..].iter().find(|w| !MODIFIERS.contains(&w.as_str()));
            match object {
                Some(object) => format!("{first} {object}"),
                None => first.clone(),
            }
        }
        "WITH" | "VALUES" if returns_rows => "SELECT".into(),
        // TODO: WITH ... INSERT/UPDATE/DELETE without RETURNING: find the
        // statement after the CTEs to report its row count.
        _ => first.clone(),
    }
}

/// Whether `changes()` counts this command's rows.
pub fn counts_changes(command: &str) -> bool {
    matches!(command, "INSERT" | "UPDATE" | "DELETE" | "REPLACE")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands() {
        assert_eq!(command_of("select 1", true), "SELECT");
        assert_eq!(
            command_of("-- hi\n/* x */ insert into t values (1)", false),
            "INSERT"
        );
        assert_eq!(command_of("CREATE TEMP TABLE t (a)", false), "CREATE TABLE");
        assert_eq!(
            command_of("create unique index i on t(a)", false),
            "CREATE INDEX"
        );
        assert_eq!(command_of("drop view v", false), "DROP VIEW");
        assert_eq!(
            command_of("with x as (select 1) select * from x", true),
            "SELECT"
        );
        assert_eq!(command_of("pragma table_info(t)", true), "PRAGMA");
        assert_eq!(command_of("", false), "");
    }
}
