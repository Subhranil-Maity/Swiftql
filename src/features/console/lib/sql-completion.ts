// Autocomplete from the connected database. lang-sql completes table names,
// `schema.table`, `table.column` and `alias.column` from the namespace built
// here. It never offers plain column names, so `tableColumns` adds the columns
// of the tables the current statement reads or writes.
import type { Completion, CompletionContext, CompletionResult, CompletionSource } from "@codemirror/autocomplete";
import { syntaxTree } from "@codemirror/language";
import type { SQLNamespace } from "@codemirror/lang-sql";

import { currentStatement } from "@/features/console/lib/statement-markers";
import type { Column, Table, TableKind } from "@/lib/ipc";
import type { Catalog } from "@/stores/schema-catalog";

const KIND_LABEL: Record<TableKind, string> = {
  table: "table",
  view: "view",
  materializedView: "materialized view",
  foreignTable: "foreign table",
  partitionedTable: "partitioned table",
};

/** Names Postgres would fold or reject unquoted get quotes when inserted. */
// TODO: also quote reserved words used as names (a column called "user", "order")
function insertText(name: string): string | undefined {
  return /^[a-z_][a-z0-9_$]*$/.test(name) ? undefined : `"${name.replace(/"/g, '""')}"`;
}

function columnCompletion(column: Column, detail: string, boost = 0): Completion {
  return { label: column.name, apply: insertText(column.name), type: "property", detail, boost };
}

function tableCompletion(table: Table): Completion {
  return {
    label: table.name,
    apply: insertText(table.name),
    type: table.kind === "table" || table.kind === "partitionedTable" ? "class" : "interface",
    detail: KIND_LABEL[table.kind],
  };
}

/** lang-sql splits keys on dots, so a dot in a name has to be escaped. */
const key = (name: string) => name.replace(/\./g, "\\.");

/** The catalog in lang-sql's shape: schema → table → columns. */
export function catalogNamespace(catalog: Catalog): SQLNamespace {
  const namespace: Record<string, SQLNamespace> = {};
  for (const schema of catalog.schemas) {
    const tables: Record<string, SQLNamespace> = {};
    for (const table of schema.tables ?? []) {
      tables[key(table.name)] = {
        self: tableCompletion(table),
        children: (table.columns ?? []).map((c) => columnCompletion(c, c.dataType)),
      };
    }
    namespace[key(schema.name)] = {
      self: { label: schema.name, apply: insertText(schema.name), type: "namespace", detail: "schema" },
      children: tables,
    };
  }
  return namespace;
}

/** Unqualified table names resolve against the first schema on the search path. */
export function defaultSchema(catalog: Catalog): string | undefined {
  return catalog.searchPath[0];
}

// One name: "quoted ""name""" or a bare identifier.
const NAME = String.raw`(?:"(?:[^"]|"")+"|[\p{L}_][\p{L}\p{N}_$]*)`;
const TABLE_REF = new RegExp(String.raw`(${NAME})(?:\s*\.\s*(${NAME}))?`, "uy");
const INTRO = /\b(?:from|join|update|into)\s+(?:only\s+)?/giu;
// After a reference: an optional alias, then a comma for another table.
const ALIAS = new RegExp(String.raw`\s+(?:as\s+)?${NAME}`, "uy");
const COMMA = /\s*,\s*/uy;
const NOT_ALIAS = new Set(["where", "join", "on", "using", "set", "group", "order", "limit", "left", "right", "inner", "outer", "full", "cross", "natural", "lateral", "values", "select", "returning", "default", "union", "having", "window", "offset", "fetch", "for"]);

/** Postgres folds unquoted names to lower case. */
function unquote(name: string): string {
  return name.startsWith('"') ? name.slice(1, -1).replace(/""/g, '"') : name.toLowerCase();
}

/** Tables named after FROM, JOIN, UPDATE and INTO, as [schema, table] or [table]. */
export function referencedTables(sql: string): string[][] {
  const refs: string[][] = [];
  for (const intro of sql.matchAll(INTRO)) {
    let pos = (intro.index ?? 0) + intro[0].length;
    for (;;) {
      TABLE_REF.lastIndex = pos;
      const ref = TABLE_REF.exec(sql);
      if (!ref) break;
      refs.push(ref[2] ? [unquote(ref[1]), unquote(ref[2])] : [unquote(ref[1])]);
      pos = TABLE_REF.lastIndex;

      ALIAS.lastIndex = pos;
      const alias = ALIAS.exec(sql);
      if (alias && !NOT_ALIAS.has(alias[0].trim().replace(/^as\s+/i, "").toLowerCase())) {
        pos = ALIAS.lastIndex;
      }
      COMMA.lastIndex = pos;
      if (!COMMA.exec(sql)) break;
      pos = COMMA.lastIndex;
    }
  }
  return refs;
}

function findTable(catalog: Catalog, ref: string[]): Table | undefined {
  const [schemaName, tableName] = ref.length === 2 ? ref : [null, ref[0]];
  const schemas = schemaName ? [schemaName] : catalog.searchPath;
  for (const name of schemas) {
    const table = catalog.schemas.find((s) => s.name === name)?.tables?.find((t) => t.name === tableName);
    if (table) return table;
  }
  return undefined;
}

/** Columns of the tables the statement at the cursor uses, offered without a table prefix. */
export function tableColumns(getCatalog: () => Catalog | undefined): CompletionSource {
  return (context: CompletionContext): CompletionResult | null => {
    const catalog = getCatalog();
    if (!catalog) return null;

    const word = context.matchBefore(/[\p{L}\p{N}_$]+/u);
    if (!word && !context.explicit) return null;
    const from = word?.from ?? context.pos;
    // After a dot, lang-sql completes that table's or alias's columns.
    if (context.state.sliceDoc(from - 1, from) === ".") return null;
    const node = syntaxTree(context.state).resolveInner(context.pos, -1);
    if (/String|Comment/.test(node.name)) return null;

    const statement = currentStatement(context.state);
    const sql = statement
      ? context.state.sliceDoc(statement.from, statement.to)
      : context.state.doc.toString();

    const options: Completion[] = [];
    const seen = new Set<Table>();
    for (const ref of referencedTables(sql)) {
      const table = findTable(catalog, ref);
      if (!table || seen.has(table)) continue;
      seen.add(table);
      for (const column of table.columns ?? []) {
        // Ranked above keywords and table names that match as well.
        options.push(columnCompletion(column, `${column.dataType} · ${table.name}`, 2));
      }
    }
    return options.length ? { from, options, validFor: /^[\p{L}\p{N}_$]*$/u } : null;
  };
}
