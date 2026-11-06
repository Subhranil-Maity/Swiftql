// The Tables view's search: by table name, or by "schema.table".
import { entryKey, type TableEntry } from "@/features/tables/model/table-entry";
import type { SchemaModel } from "@/stores/schema-model";

export type SchemaGroup = {
  schema: string;
  /** Tables that match, in name order. */
  entries: TableEntry[];
  /** Tables in the schema before filtering. */
  total: number;
};

/** Case-insensitive; a query with a dot matches against "schema.table". */
export function matchesQuery(schema: string, table: string, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  if (table.toLowerCase().includes(q)) return true;
  return q.includes(".") && `${schema}.${table}`.toLowerCase().includes(q);
}

/** Schemas with their matching tables. While searching, schemas without a match drop out. */
export function groupTables(model: SchemaModel, query: string): SchemaGroup[] {
  const searching = query.trim() !== "";
  const groups: SchemaGroup[] = [];
  for (const schema of model.schemas) {
    const tables = schema.tables ?? [];
    const entries = tables
      .filter((t) => matchesQuery(schema.name, t.name, query))
      .map((table) => ({ key: entryKey(schema.name, table.name), schema: schema.name, table }));
    if (searching && entries.length === 0) continue;
    groups.push({ schema: schema.name, entries, total: tables.length });
  }
  return groups;
}
