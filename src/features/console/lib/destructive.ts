// Statements that destroy data at once: DROP, TRUNCATE, and DELETE / UPDATE
// without WHERE. Checked before a run when Settings → Query & results →
// "Confirm destructive statements" is on.

/** The statement with comments, strings and quoted names blanked out. */
function code(sql: string): string {
  return sql
    .replace(/--[^\n]*/g, " ")
    .replace(/\/\*[\s\S]*?\*\//g, " ")
    .replace(/\$([A-Za-z_][A-Za-z0-9_]*)?\$[\s\S]*?\$\1\$/g, " ")
    .replace(/[Ee]'(?:\\.|''|[^'\\])*'/g, " ")
    .replace(/'(?:''|[^'])*'/g, " ")
    .replace(/"(?:""|[^"])*"/g, " x ");
}

/** Why `sql` is destructive, or null. */
export function destructiveReason(sql: string): string | null {
  const text = code(sql).replace(/\s+/g, " ").trim();
  // Skip a leading WITH ... so "WITH x AS (...) DELETE FROM t" is checked too.
  const body = text.replace(/^with\b.*?\)\s*(?=(delete|update|insert|select|merge)\b)/i, "");
  if (/^drop\s+(table|schema|database|view|materialized\s+view|index|sequence|function|type|extension)\b/i.test(body)) {
    return "drops an object";
  }
  if (/^truncate\b/i.test(body)) return "empties a table";
  if (/^delete\s+from\b/i.test(body) && !/\bwhere\b/i.test(body)) return "deletes every row";
  if (/^update\b/i.test(body) && !/\bwhere\b/i.test(body)) return "updates every row";
  return null;
}

export type DestructiveStatement = { sql: string; reason: string };

export function destructiveStatements(statements: string[]): DestructiveStatement[] {
  return statements.flatMap((sql) => {
    const reason = destructiveReason(sql);
    return reason ? [{ sql, reason }] : [];
  });
}
