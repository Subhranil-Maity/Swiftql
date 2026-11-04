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

/// Words outside parentheses, strings, quoted names and comments, uppercased.
/// `CREATE TABLE t (a int) AS SELECT 1` gives CREATE TABLE T AS SELECT 1.
/// TODO: an E'' string with a backslash-escaped quote ends the string early.
fn top_level_words(sql: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut depth = 0usize;
    let mut rest = sql;
    while let Some(c) = rest.chars().next() {
        if rest.starts_with("--") || rest.starts_with("/*") {
            rest = skip_space_and_comments(rest);
            continue;
        }
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '\'' | '"' => {
                // A doubled quote inside is an escape: scanning just resumes after it.
                rest = rest[1..].find(c).map_or("", |i| &rest[i + 2..]);
                continue;
            }
            '$' => {
                if let Some(close) = rest[1..].find('$') {
                    let tag = &rest[..close + 2]; // "$$" or "$name$"
                    let name = &tag[1..tag.len() - 1];
                    if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        let body = &rest[tag.len()..];
                        rest = body.find(tag).map_or("", |i| &body[i + tag.len()..]);
                        continue;
                    }
                }
            }
            c if c.is_alphanumeric() || c == '_' => {
                let len = rest
                    .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                    .unwrap_or(rest.len());
                if depth == 0 {
                    words.push(rest[..len].to_uppercase());
                }
                rest = &rest[len..];
                continue;
            }
            _ => {}
        }
        rest = &rest[c.len_utf8()..];
    }
    words
}

/// `CREATE TABLE ... AS` and `CREATE MATERIALIZED VIEW ... AS` fill the new
/// table and report "SELECT n"; with `WITH NO DATA` they don't.
fn fills_table(sql: &str) -> bool {
    let words = top_level_words(sql);
    let no_data = words.ends_with(&["NO".to_owned(), "DATA".to_owned()]);
    words.iter().any(|w| w == "AS") && !no_data
}

/// Command in the style Postgres reports it: "UPDATE", "CREATE TABLE",
/// "CREATE MATERIALIZED VIEW". Derived from the SQL because tokio-postgres
/// doesn't expose the server's tag; `PgDialect::command_tag` adds the count.
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
            let can_fill = matches!(tag.as_str(), "CREATE TABLE" | "CREATE MATERIALIZED VIEW");
            if can_fill && fills_table(sql) {
                return "SELECT".into();
            }
            tag
        }
        "START" => "START TRANSACTION".into(),
        "VALUES" | "TABLE" => "SELECT".into(),
        // The statement after the CTEs decides: WITH d AS (...) INSERT ... is "INSERT".
        "WITH" => top_level_words(sql)
            .into_iter()
            .find_map(|w| match w.as_str() {
                "SELECT" | "VALUES" | "TABLE" => Some("SELECT".to_owned()),
                "INSERT" | "UPDATE" | "DELETE" | "MERGE" => Some(w),
                _ => None,
            })
            .unwrap_or_else(|| first.clone()),
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
            command_tag("CREATE MATERIALIZED VIEW mv AS SELECT 1 WITH NO DATA"),
            "CREATE MATERIALIZED VIEW"
        );
        assert_eq!(command_tag("drop table if exists t"), "DROP TABLE");
        assert_eq!(command_tag("start transaction"), "START TRANSACTION");
        assert_eq!(command_tag("   "), "");
    }

    #[test]
    fn tags_that_report_select() {
        assert_eq!(command_tag("CREATE TABLE t AS SELECT 1"), "SELECT");
        assert_eq!(
            command_tag("create temp table t (a) as values (1)"),
            "SELECT"
        );
        assert_eq!(
            command_tag("CREATE MATERIALIZED VIEW mv AS SELECT 1"),
            "SELECT"
        );
        // AS inside parentheses, strings or comments doesn't count
        assert_eq!(
            command_tag("CREATE TABLE t (a int, b int GENERATED ALWAYS AS (a) STORED)"),
            "CREATE TABLE"
        );
        assert_eq!(
            command_tag("CREATE TABLE t (a text DEFAULT ' as ') -- as"),
            "CREATE TABLE"
        );
        assert_eq!(command_tag("VALUES (1)"), "SELECT");
        assert_eq!(command_tag("TABLE t"), "SELECT");
    }

    #[test]
    fn cte_tags() {
        assert_eq!(
            command_tag("WITH x AS (SELECT 1) SELECT * FROM x"),
            "SELECT"
        );
        assert_eq!(
            command_tag("with d as (delete from t returning *) insert into u select * from d"),
            "INSERT"
        );
        assert_eq!(
            command_tag(
                "WITH RECURSIVE r(n) AS (VALUES (1) UNION SELECT n + 1 FROM r) UPDATE t SET a = 1"
            ),
            "UPDATE"
        );
        assert_eq!(
            command_tag("WITH x AS (SELECT $$ ) $$) DELETE FROM t"),
            "DELETE"
        );
    }

    #[test]
    fn words_outside_parens() {
        assert_eq!(
            top_level_words(r#"a ("b" c) 'd''e' f /* g */ $x$ h $x$ i"#),
            ["A", "F", "I"]
        );
    }
}
