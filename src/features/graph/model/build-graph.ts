// The schema model as graph nodes and edges. Pure: positions are added later
// (layout.ts, or the user's saved drags).
import type { Edge, Node } from "@xyflow/react";

import { ghostNodeSize, type NodeSize, nodeColumns, tableNodeSize } from "@/features/graph/model/node-size";
import { entryKey, type TableEntry } from "@/features/tables/model/table-entry";
import type { ForeignKey } from "@/lib/ipc";
import type { SchemaModel } from "@/stores/schema-model";

export type TableNodeData = {
  entry: TableEntry;
  /** Shown in the header when tables from several schemas are on screen. */
  schemaLabel: string | null;
  size: NodeSize;
};

/** A table outside the selected schemas that a visible table references (or is referenced by). */
export type GhostNodeData = {
  schema: string;
  table: string;
  size: NodeSize;
};

export type TableNode = Node<TableNodeData, "table">;
export type GhostNode = Node<GhostNodeData, "ghost">;
export type GraphNode = TableNode | GhostNode;

export type FkEdgeData = {
  fk: ForeignKey;
  /** The referencing table: the "many" end. */
  fromKey: string;
};
export type FkEdge = Edge<FkEdgeData, "fk">;

/** Ghost cards (and tables without columns) have one handle per side, on the card itself. */
export const cardHandle = (side: "l" | "r") => `card:${side}`;

/** Handle ids: every column row has a left (target) and right (source) handle. */
export const leftHandle = (column: string) => `col:${column}:l`;
export const rightHandle = (column: string) => `col:${column}:r`;

export type GraphData = { nodes: GraphNode[]; edges: FkEdge[] };

/**
 * One node per table in `schemas`, one edge per foreign key. An edge runs
 * from the referenced table (the "one" side, laid out to the left) to the
 * referencing one, leaving and entering at the exact column rows.
 */
export function buildGraph(model: SchemaModel, schemas: ReadonlySet<string>): GraphData {
  const multiSchema = schemas.size > 1;
  const nodes = new Map<string, GraphNode>();
  const tables = new Map<string, TableEntry>();

  for (const schema of model.schemas) {
    for (const table of schema.tables ?? []) {
      const key = entryKey(schema.name, table.name);
      tables.set(key, { key, schema: schema.name, table });
      if (!schemas.has(schema.name)) continue;
      const schemaLabel = multiSchema ? schema.name : null;
      nodes.set(key, {
        id: key,
        type: "table",
        position: { x: 0, y: 0 },
        data: { entry: { key, schema: schema.name, table }, schemaLabel, size: tableNodeSize(table, schemaLabel) },
      });
    }
  }

  const ghost = (schema: string, table: string): GhostNode => ({
    id: entryKey(schema, table),
    type: "ghost",
    position: { x: 0, y: 0 },
    data: { schema, table, size: ghostNodeSize() },
  });

  /** The handle a column's edge attaches to; a hidden column falls back to the card's first row. */
  const columnHandle = (key: string, column: string | undefined, side: "l" | "r") => {
    const node = nodes.get(key);
    if (!node || node.type === "ghost") return cardHandle(side);
    const { shown } = nodeColumns(node.data.entry.table);
    const name = shown.some((c) => c.name === column) ? column : shown[0]?.name;
    return name ? (side === "l" ? leftHandle(name) : rightHandle(name)) : cardHandle(side);
  };

  const edges: FkEdge[] = [];
  for (const entry of tables.values()) {
    for (const fk of entry.table.foreignKeys ?? []) {
      const refKey = entryKey(fk.refSchema, fk.refTable);
      const fromShown = schemas.has(entry.schema);
      const toShown = schemas.has(fk.refSchema);
      if (!fromShown && !toShown) continue;
      // One end is outside the selected schemas: show it as a stub so the reference isn't lost.
      if (!fromShown && !nodes.has(entry.key)) nodes.set(entry.key, ghost(entry.schema, entry.table.name));
      if (!toShown && !nodes.has(refKey)) nodes.set(refKey, ghost(fk.refSchema, fk.refTable));
      edges.push({
        id: `${entry.key}/${fk.name}`,
        type: "fk",
        source: refKey,
        sourceHandle: columnHandle(refKey, fk.refColumns[0], "r"),
        target: entry.key,
        targetHandle: columnHandle(entry.key, fk.columns[0], "l"),
        data: { fk, fromKey: entry.key },
      });
    }
  }

  return { nodes: [...nodes.values()], edges };
}
