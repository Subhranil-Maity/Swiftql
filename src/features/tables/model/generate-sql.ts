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

/** "New INSERT": one row, DEFAULT where the column has one, else NULL, each labelled. */
export function insertSql(schema: string, table: string, columns: Column[]): string {
  const target = displayQualifiedName(schema, table);
  if (columns.length === 0) return `INSERT INTO ${target} DEFAULT VALUES;\n`;
  const names = columns.map((c) => `  ${ident(c.name)}`).join(",\n");
  const values = columns
    .map((c, i) => {
      const value = c.default != null ? "DEFAULT" : "NULL";
      const comma = i < columns.length - 1 ? "," : "";
      return `  ${value}${comma} -- ${c.name} ${c.dataType}`;
    })
    .join("\n");
  return `INSERT INTO ${target} (\n${names}\n) VALUES (\n${values}\n);\n`;
}
