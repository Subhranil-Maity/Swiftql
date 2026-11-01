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
}
