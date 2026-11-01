/// First `n` keywords of a statement, uppercased. Skips whitespace and
/// comments (`--`, nested `/* */`); stops at the first non-word token.
pub fn leading_keywords(sql: &str, n: usize) -> Vec<String> {
    let mut words = Vec::new();
    let mut rest = sql;
    while words.len() < n {
        rest = skip_space_and_comments(rest);
        let len = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if len == 0 {
            break; // '(', ';', a quote, end of input...
        }
        words.push(rest[..len].to_ascii_uppercase());
        rest = &rest[len..];
    }
    words
}

fn skip_space_and_comments(mut s: &str) -> &str {
    loop {
        s = s.trim_start();
        if let Some(after) = s.strip_prefix("--") {
            s = after.find('\n').map_or("", |i| &after[i + 1..]);
        } else if s.starts_with("/*") {
            s = skip_block_comment(s);
        } else {
            return s;
        }
    }
}

/// `s` starts with "/*". Postgres block comments nest.
fn skip_block_comment(s: &str) -> &str {
    let mut depth = 0;
    let mut i = 0;
    let bytes = s.as_bytes();
    while i + 1 < bytes.len() {
        match &bytes[i..i + 2] {
            b"/*" => {
                depth += 1;
                i += 2;
            }
            b"*/" => {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    return &s[i..];
                }
            }
            _ => i += 1,
        }
    }
    "" // unterminated comment
}

/// Words after CREATE / DROP / ALTER that Postgres leaves out of the command tag.
const MODIFIERS: &[&str] = &[
    "OR",
    "REPLACE",
    "UNIQUE",
    "TEMP",
    "TEMPORARY",
    "UNLOGGED",
    "GLOBAL",
    "LOCAL",
    "RECURSIVE",
    "TRUSTED",
    "PROCEDURAL",
    "DEFAULT",
    "CONCURRENTLY",
];

/// Objects named by two words ("MATERIALIZED VIEW", "FOREIGN TABLE", ...).
const TWO_WORD_OBJECTS: &[&str] = &["MATERIALIZED", "FOREIGN", "EVENT", "ACCESS", "USER"];

/// Command tag in the style Postgres reports: "UPDATE", "CREATE TABLE",
/// "CREATE MATERIALIZED VIEW". Derived from the SQL because tokio-postgres
/// doesn't expose the server's tag.
/// TODO: a data-modifying CTE (WITH ... INSERT) reports "WITH"; the server says "INSERT".
pub fn command_tag(sql: &str) -> String {
    let words = leading_keywords(sql, 8);
    let Some(first) = words.first() else {
        return String::new();
    };
    match first.as_str() {
        "CREATE" | "DROP" | "ALTER" => {
            let mut object = words[1..]
                .iter()
                .skip_while(|w| MODIFIERS.contains(&w.as_str()));
            let mut tag = first.clone();
            if let Some(word) = object.next() {
                tag.push(' ');
                tag.push_str(word);
                if TWO_WORD_OBJECTS.contains(&word.as_str()) {
                    if let Some(next) = object.next() {
                        tag.push(' ');
                        tag.push_str(next);
                    }
                }
            }
            tag
        }
        "START" => "START TRANSACTION".into(),
        _ => first.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_skip_comments() {
        let sql = "-- note\n /* outer /* inner */ still */ select * from t";
        assert_eq!(leading_keywords(sql, 2), ["SELECT"]); // stops at '*'
        assert_eq!(
            leading_keywords("  rollback to sp1", 3),
            ["ROLLBACK", "TO", "SP1"]
        );
        assert!(leading_keywords("/* unterminated", 1).is_empty());
    }

    #[test]
    fn tags() {
        assert_eq!(command_tag("update t set a = 1"), "UPDATE");
        assert_eq!(command_tag("CREATE TABLE t (id int)"), "CREATE TABLE");
        assert_eq!(
            command_tag("create or replace function f()"),
            "CREATE FUNCTION"
        );
        assert_eq!(
            command_tag("CREATE UNIQUE INDEX i ON t (a)"),
            "CREATE INDEX"
        );
        assert_eq!(command_tag("create temp table x(a int)"), "CREATE TABLE");
        assert_eq!(
            command_tag("CREATE MATERIALIZED VIEW mv AS SELECT 1"),
            "CREATE MATERIALIZED VIEW"
        );
        assert_eq!(command_tag("drop table if exists t"), "DROP TABLE");
        assert_eq!(command_tag("start transaction"), "START TRANSACTION");
        assert_eq!(command_tag("   "), "");
    }
}
