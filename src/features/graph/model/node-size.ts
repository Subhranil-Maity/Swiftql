// A table card's size, worked out before it renders so elk can lay the graph
// out in one pass. The card's CSS follows the same numbers (table-node.tsx).
import type { Column, Table } from "@/lib/ipc";

/** Every column row is exactly this tall: the card's baseline grid. */
export const ROW_HEIGHT = 24;
export const HEADER_HEIGHT = 32;
/** Padding above and below the column list. */
export const BODY_PADDING_Y = 4;
const BORDERS = 2;
const MIN_WIDTH = 180;
const MAX_WIDTH = 340;
/** Columns a card shows before "+N more"; key columns always show. */
export const MAX_NODE_COLUMNS = 24;

const GHOST_WIDTH = 180;
const GHOST_HEIGHT = 36;

let measurer: CanvasRenderingContext2D | null = null;
function textWidth(text: string, font: string): number {
  measurer ??= document.createElement("canvas").getContext("2d");
  if (!measurer) return text.length * 7.5;
  measurer.font = font;
  return measurer.measureText(text).width;
}

const NAME_FONT = "12px 'Geist Variable', sans-serif";
const TITLE_FONT = "500 12.5px 'Geist Variable', sans-serif";
const TYPE_FONT = "11px 'Geist Mono Variable', monospace";

/** The columns a card lists: keys first if the table is too wide to show everything. */
export function nodeColumns(table: Table): { shown: Column[]; hidden: number } {
  const columns = table.columns ?? [];
  if (columns.length <= MAX_NODE_COLUMNS) return { shown: columns, hidden: 0 };
  const isKey = (c: Column) =>
    (table.primaryKey?.includes(c.name) ?? false) ||
    (table.foreignKeys?.some((fk) => fk.columns.includes(c.name)) ?? false);
  const keys = columns.filter(isKey);
  const rest = columns.filter((c) => !isKey(c)).slice(0, Math.max(0, MAX_NODE_COLUMNS - keys.length));
  const keep = new Set([...keys, ...rest]);
  // Keep the table's own column order.
  const shown = columns.filter((c) => keep.has(c));
  return { shown, hidden: columns.length - shown.length };
}

export type NodeSize = { width: number; height: number };

export function tableNodeSize(table: Table, schemaLabel: string | null): NodeSize {
  const { shown, hidden } = nodeColumns(table);
  // Header: icon, gap, name, schema label, padding.
  let widest = 14 + 8 + textWidth(table.name, TITLE_FONT) + 24;
  if (schemaLabel) widest += textWidth(schemaLabel, TYPE_FONT) + 8;
  for (const column of shown) {
    // Gutter, gap, name, gap, type, nullable mark, padding.
    const row = 14 + 6 + textWidth(column.name, NAME_FONT) + 12 + textWidth(column.dataType, TYPE_FONT) + 10 + 20;
    widest = Math.max(widest, row);
  }
  const rows = shown.length + (hidden > 0 ? 1 : 0);
  return {
    width: Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, widest))),
    height: HEADER_HEIGHT + BODY_PADDING_Y * 2 + rows * ROW_HEIGHT + BORDERS,
  };
}

export function ghostNodeSize(): NodeSize {
  return { width: GHOST_WIDTH, height: GHOST_HEIGHT };
}
