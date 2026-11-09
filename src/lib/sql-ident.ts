// Identifiers in generated SQL. `quoteIdent` and `qualifiedName` match the
// Postgres dialect in crates/driver-postgres/src/dialect.rs. SQLite reads
// the same double-quoted form, and quoting more than it needs is harmless
// there (its names ignore case either way).

/** Always quoted: `my "col"` → `"my ""col"""`. */
export function quoteIdent(name: string): string {
  return `"${name.replace(/"/g, '""')}"`;
}

/** `"schema"."table"`. Also unique per table, so it doubles as a key. */
export function qualifiedName(schema: string, table: string): string {
  return `${quoteIdent(schema)}.${quoteIdent(table)}`;
}

// Postgres reserved keywords (and the reserved-but-allowed-as-function ones):
// these need quotes even when they're plain lowercase.
const RESERVED = new Set(
  (
    "all analyse analyze and any array as asc asymmetric authorization binary both case cast check " +
    "collate collation column concurrently constraint create cross current_catalog current_date " +
    "current_role current_schema current_time current_timestamp current_user default deferrable desc " +
    "distinct do else end except false fetch for foreign freeze from full grant group having ilike in " +
    "initially inner intersect into is isnull join lateral leading left like limit localtime " +
    "localtimestamp natural not notnull null offset on only or order outer overlaps placing primary " +
    "references returning right select session_user similar some symmetric system_user table tablesample " +
    "then to trailing true union unique user using variadic verbose when where window with"
  ).split(" "),
);

/** Quoted only when it has to be, so generated SQL stays readable: `orders`, `"Order"`, `"user"`. */
export function ident(name: string): string {
  return /^[a-z_][a-z0-9_$]*$/.test(name) && !RESERVED.has(name) ? name : quoteIdent(name);
}

/** `schema.table` with `ident` quoting. */
export function displayQualifiedName(schema: string, table: string): string {
  return `${ident(schema)}.${ident(table)}`;
}
