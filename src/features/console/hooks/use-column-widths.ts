import { useCallback, useEffect, useMemo, useState } from "react";

import { SORT_ARROW_SPACE } from "@/features/console/lib/draw-header";
import { MONO, SANS } from "@/features/console/lib/grid-theme";
import { formatCell } from "@/features/console/model/cell-format";
import type { ColumnMeta, DbValue } from "@/lib/ipc";

const MIN_WIDTH = 64;
const MAX_WIDTH = 360;
/** Rows looked at to size a column. */
const SAMPLE_ROWS = 50;
const PADDING = 20;

let measurer: CanvasRenderingContext2D | null = null;
function textWidth(text: string, font: string): number {
  measurer ??= document.createElement("canvas").getContext("2d");
  if (!measurer) return text.length * 8;
  measurer.font = font;
  return measurer.measureText(text).width;
}

/** Initial widths from the two-line header and the first rows. */
function autoWidths(columns: ColumnMeta[], rows: DbValue[][]): number[] {
  const sample = rows.slice(0, SAMPLE_ROWS);
  return columns.map((column, i) => {
    // Room for the sort arrow, so sorting doesn't truncate the name.
    let widest =
      Math.max(textWidth(column.name, `500 12px ${SANS}`), textWidth(column.nativeType, `11px ${MONO}`)) +
      SORT_ARROW_SPACE;
    for (const row of sample) {
      const text = formatCell(row[i] ?? { type: "null" }).display.slice(0, 80);
      widest = Math.max(widest, textWidth(text, `12.5px ${MONO}`));
    }
    return Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, widest + PADDING)));
  });
}

/** Column widths by source index: sized once rows arrive, then the user's resizing wins. */
export function useColumnWidths(columns: ColumnMeta[], rows: DbValue[][], fontsReady: boolean) {
  const [resized, setResized] = useState<Record<number, number>>({});
  useEffect(() => setResized({}), [columns]);

  const hasRows = rows.length > 0;
  // Sized from the first rows only, not on every batch.
  const auto = useMemo(() => autoWidths(columns, rows), [columns, hasRows, fontsReady]);

  const widthOf = useCallback((column: number) => resized[column] ?? auto[column], [resized, auto]);
  const resize = useCallback(
    (column: number, width: number) => setResized((r) => ({ ...r, [column]: width })),
    [],
  );
  return { widthOf, resize };
}
