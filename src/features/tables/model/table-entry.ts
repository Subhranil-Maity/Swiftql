// A table together with the schema it lives in: what the Tables list, the
// details sheet and the graph pass around.
import type { Column, Table } from "@/lib/ipc";
import { qualifiedName } from "@/lib/sql-ident";
import type { SchemaModel } from "@/stores/schema-model";

export type TableEntry = {
  /** `"schema"."table"`: unique, and safe as a React key or graph node id. */
  key: string;
  schema: string;
  table: Table;
};

export function entryKey(schema: string, table: string): string {
  return qualifiedName(schema, table);
}

/** Every table in the model, schema by schema. */
export function tableEntries(model: SchemaModel): TableEntry[] {
  return model.schemas.flatMap((schema) =>
    (schema.tables ?? []).map((table) => ({
      key: entryKey(schema.name, table.name),
      schema: schema.name,
      table,
    })),
  );
}

export function findEntry(model: SchemaModel | undefined, key: string): TableEntry | undefined {
  if (!model) return undefined;
  for (const schema of model.schemas) {
    for (const table of schema.tables ?? []) {
      if (entryKey(schema.name, table.name) === key) {
        return { key, schema: schema.name, table };
      }
    }
  }
  return undefined;
}

/** What a column takes part in, for its glyph and tint. */
export type ColumnRole = { primary: boolean; foreign: boolean };

export function columnRole(table: Table, column: Column): ColumnRole {
  return {
    primary: table.primaryKey?.includes(column.name) ?? false,
    foreign: table.foreignKeys?.some((fk) => fk.columns.includes(column.name)) ?? false,
  };
}
