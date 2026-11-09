use driver_core::dialect::Dialect;

pub struct SqliteDialect;

impl Dialect for SqliteDialect {
    fn quote_ident(&self, name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }

    fn quote_literal(&self, value: &str) -> String {
        // No backslash escapes in SQLite strings.
        format!("'{}'", value.replace('\'', "''"))
    }

    fn qualified_name(&self, schema: &str, table: &str) -> String {
        format!("{}.{}", self.quote_ident(schema), self.quote_ident(table))
    }

    fn placeholder(&self, index: usize) -> String {
        format!("?{index}")
    }

    /// The command with its row count where one is meaningful; "CREATE TABLE" alone.
    fn command_tag(&self, command: &str, rows: u64) -> String {
        match command {
            "SELECT" | "INSERT" | "UPDATE" | "DELETE" | "REPLACE" => format!("{command} {rows}"),
            _ => command.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        let d = SqliteDialect;
        assert_eq!(d.quote_ident(r#"my "col""#), r#""my ""col""""#);
        assert_eq!(d.quote_literal(r"it's C:\tmp"), r"'it''s C:\tmp'");
        assert_eq!(d.qualified_name("main", "User"), r#""main"."User""#);
        assert_eq!(d.placeholder(2), "?2");
        assert_eq!(d.command_tag("INSERT", 3), "INSERT 3");
        assert_eq!(d.command_tag("CREATE TABLE", 0), "CREATE TABLE");
    }
}
