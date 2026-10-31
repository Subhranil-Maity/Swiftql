pub trait Dialect: Send + Sync {
    fn quote_ident(&self, name: &str) -> String; // PG: "my col"   MySQL: `my col`
    fn quote_literal(&self, value: &str) -> String; // for display/scripts only
    fn qualified_name(&self, schema: &str, table: &str) -> String;
    /// `index` is 1-based: `placeholder(1)` is the first parameter.
    fn placeholder(&self, index: usize) -> String; // PG: $1   MySQL: ?
}
