use driver_core::dialect::Dialect;

pub struct PgDialect;

impl Dialect for PgDialect {
    fn quote_ident(&self, name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }

    fn quote_literal(&self, value: &str) -> String {
        let escaped = value.replace('\'', "''");
        // Same as Postgres' quote_literal(): E'' keeps backslashes literal
        // whatever standard_conforming_strings is set to.
        if escaped.contains('\\') {
            format!("E'{}'", escaped.replace('\\', "\\\\"))
        } else {
            format!("'{escaped}'")
        }
    }

    fn qualified_name(&self, schema: &str, table: &str) -> String {
        format!("{}.{}", self.quote_ident(schema), self.quote_ident(table))
    }

    fn placeholder(&self, index: usize) -> String {
        format!("${index}")
    }

    /// The tag psql prints. INSERT keeps the historic OID column (always 0);
    /// commands without a count, e.g. CREATE TABLE, are just the command.
    fn command_tag(&self, command: &str, rows: u64) -> String {
        match command {
            "INSERT" => format!("INSERT 0 {rows}"),
            "SELECT" | "UPDATE" | "DELETE" | "MERGE" | "FETCH" | "MOVE" | "COPY" => {
                format!("{command} {rows}")
            }
            _ => command.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        let d = PgDialect;
        assert_eq!(d.quote_ident("users"), r#""users""#);
        assert_eq!(d.quote_ident(r#"my "col""#), r#""my ""col""""#);
        assert_eq!(d.quote_literal("it's"), "'it''s'");
        assert_eq!(d.quote_literal(r"C:\tmp"), r"E'C:\\tmp'");
        assert_eq!(d.qualified_name("public", "User"), r#""public"."User""#);
        assert_eq!(d.placeholder(1), "$1");
    }

    #[test]
    fn command_tags() {
        let d = PgDialect;
        assert_eq!(d.command_tag("INSERT", 3), "INSERT 0 3");
        assert_eq!(d.command_tag("SELECT", 1000), "SELECT 1000");
        assert_eq!(d.command_tag("UPDATE", 0), "UPDATE 0");
        assert_eq!(d.command_tag("MERGE", 2), "MERGE 2");
        assert_eq!(d.command_tag("CREATE TABLE", 0), "CREATE TABLE");
        assert_eq!(d.command_tag("BEGIN", 0), "BEGIN");
    }
}
