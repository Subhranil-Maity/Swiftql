// SQL the Tables and Graph views put in a new console tab.
import { displayQualifiedName, ident } from "@/lib/sql-ident";
import type { Column } from "@/lib/ipc";

/** What "Open data" runs. Every row: the console pages through the held result, which stops at the memory limit in Settings. */
export function selectAllSql(schema: string, table: string): string {
  return `SELECT *\nFROM ${displayQualifiedName(schema, table)};\n`;
}

/** "New SELECT": every column spelled out, ready to trim. */
export function selectColumnsSql(schema: string, table: string, columns: Column[]): string {
  if (columns.length === 0) return selectAllSql(schema, table);
  const list = columns.map((c) => `  ${ident(c.name)}`).join(",\n");
  return `SELECT\n${list}\nFROM ${displayQualifiedName(schema, table)};\n`;
}

/** Drivers without the DEFAULT keyword in VALUES. */
const NO_DEFAULT_KEYWORD = new Set(["sqlite"]);

/**
 * "New INSERT": one row, DEFAULT where the column has one, else NULL, each
 * labelled. Where VALUES can't say DEFAULT (SQLite), columns with a default
 * are left out of the list instead, which gives them their default.
 */
export function insertSql(schema: string, table: string, columns: Column[], driverId: string): string {
  const target = displayQualifiedName(schema, table);
  const omitDefaults = NO_DEFAULT_KEYWORD.has(driverId);
  const listed = omitDefaults ? columns.filter((c) => c.default == null) : columns;
  if (listed.length === 0) return `INSERT INTO ${target} DEFAULT VALUES;\n`;
  const names = listed.map((c) => `  ${ident(c.name)}`).join(",\n");
  const values = listed
    .map((c, i) => {
      const value = c.default != null ? "DEFAULT" : "NULL";
      const comma = i < listed.length - 1 ? "," : "";
      return `  ${value}${comma} -- ${c.name} ${c.dataType}`;
    })
    .join("\n");
  const skipped = columns.filter((c) => !listed.includes(c));
  const note = skipped.length
    ? `-- Left out, so they get their default: ${skipped.map((c) => c.name).join(", ")}\n`
    : "";
  return `${note}INSERT INTO ${target} (\n${names}\n) VALUES (\n${values}\n);\n`;
}
