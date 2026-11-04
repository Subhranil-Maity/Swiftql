// SQL the Tables and Graph views put in a new console tab.
import { displayQualifiedName, ident } from "@/lib/sql-ident";
import type { Column } from "@/lib/ipc";

/** Rows "Open data" fetches. */
// TODO: page through more than this, sorted on the server (see TODO.md)
export const OPEN_DATA_LIMIT = 500;

/** What "Open data" runs. */
export function selectAllSql(schema: string, table: string): string {
  return `SELECT *\nFROM ${displayQualifiedName(schema, table)}\nLIMIT ${OPEN_DATA_LIMIT};\n`;
}

/** "New SELECT": every column spelled out, ready to trim. */
export function selectColumnsSql(schema: string, table: string, columns: Column[]): string {
  if (columns.length === 0) return selectAllSql(schema, table);
  const list = columns.map((c) => `  ${ident(c.name)}`).join(",\n");
  return `SELECT\n${list}\nFROM ${displayQualifiedName(schema, table)}\nLIMIT ${OPEN_DATA_LIMIT};\n`;
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
