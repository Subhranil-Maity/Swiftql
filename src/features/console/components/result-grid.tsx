import "@glideapps/glide-data-grid/dist/index.css";

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  DataEditor,
  type GridCell,
  GridCellKind,
  type GridColumn,
  type GridMouseEventArgs,
  type Item,
} from "@glideapps/glide-data-grid";

import { gridTheme } from "@/features/console/lib/grid-theme";
import { formatCell } from "@/features/console/model/cell-format";
import type { StatementResult } from "@/features/console/model/run";
import type { ColumnMeta, DbValue } from "@/lib/ipc";
import { useIsDark } from "@/stores/theme";

const MIN_WIDTH = 64;
const MAX_WIDTH = 360;
/** Rows looked at to size a column. */
const SAMPLE_ROWS = 50;

/** Fonts are drawn on canvas, so the grid redraws once they've loaded. */
function useFontsReady(): boolean {
  const [ready, setReady] = useState(document.fonts.status === "loaded");
  useEffect(() => {
    if (!ready) void document.fonts.ready.then(() => setReady(true));
  }, [ready]);
  return ready;
}

let measurer: CanvasRenderingContext2D | null = null;
function textWidth(text: string, font: string): number {
  measurer ??= document.createElement("canvas").getContext("2d");
  if (!measurer) return text.length * 8;
  measurer.font = font;
  return measurer.measureText(text).width;
}

/** Initial widths from the header and the first rows. */
function autoWidths(columns: ColumnMeta[], rows: DbValue[][]): number[] {
  const sample = rows.slice(0, SAMPLE_ROWS);
  return columns.map((column, i) => {
    let widest = textWidth(column.name, "500 12px 'Geist Variable', sans-serif") + 8;
    for (const row of sample) {
      const text = formatCell(row[i] ?? { type: "null" }).display.slice(0, 80);
      widest = Math.max(widest, textWidth(text, "12.5px 'Geist Mono Variable', monospace"));
    }
    return Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, widest + 24)));
  });
}

/** One statement's rows: read-only, virtualized, copyable with Ctrl+C. */
export function ResultGrid({ result, rowLimit }: { result: StatementResult; rowLimit: number }) {
  const isDark = useIsDark();
  const fontsReady = useFontsReady();
  // Re-read the CSS variables when the theme or fonts change.
  const theme = useMemo(() => gridTheme(), [isDark, fontsReady]);

  const columns = result.columns ?? [];
  const rows = result.rows;
  const hasRows = rows.length > 0;

  // Sized once rows arrive; the user's resizing wins after that.
  const [resized, setResized] = useState<Record<number, number>>({});
  useEffect(() => setResized({}), [columns]);
  // Sized from the first rows only, not on every batch.
  const widths = useMemo(() => autoWidths(columns, rows), [columns, hasRows, fontsReady]);

  const gridColumns = useMemo<GridColumn[]>(
    () =>
      columns.map((column, i) => ({
        id: String(i),
        title: column.name,
        width: resized[i] ?? widths[i],
      })),
    [columns, widths, resized],
  );

  const getCellContent = useCallback(
    ([col, row]: Item): GridCell => {
      const cell = formatCell(rows[row]?.[col] ?? { type: "null" });
      return {
        kind: GridCellKind.Text,
        data: cell.copy,
        displayData: cell.display,
        copyData: cell.copy,
        // TODO: a read-only overlay to see long values in full (needs glide's #portal element)
        allowOverlay: false,
        readonly: true,
        contentAlign: cell.kind === "number" ? "right" : "left",
        themeOverride: cell.kind === "null" ? { textDark: theme.textLight } : undefined,
      };
    },
    [rows, theme],
  );

  // The header's native type, shown on hover.
  const [header, setHeader] = useState<{ text: string; x: number; y: number } | null>(null);
  const onItemHovered = useCallback(
    (args: GridMouseEventArgs) => {
      if (args.kind !== "header") return setHeader(null);
      const column = columns[args.location[0]];
      if (!column) return setHeader(null);
      setHeader({
        text: column.nativeType,
        x: args.bounds.x + 8,
        y: args.bounds.y + args.bounds.height + 4,
      });
    },
    [columns],
  );

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="relative min-h-0 flex-1" onMouseLeave={() => setHeader(null)}>
        <DataEditor
          columns={gridColumns}
          rows={rows.length}
          getCellContent={getCellContent}
          getCellsForSelection
          theme={theme}
          width="100%"
          height="100%"
          rowHeight={26}
          headerHeight={30}
          rowMarkers="number"
          smoothScrollX
          smoothScrollY
          onItemHovered={onItemHovered}
          onColumnResize={(_, size, index) => setResized((r) => ({ ...r, [index]: size }))}
        />
        {!hasRows && result.status !== "running" && (
          <p className="pointer-events-none absolute inset-x-0 top-14 text-center text-muted-foreground">
            No rows
          </p>
        )}
        {header && (
          <div
            role="tooltip"
            className="pointer-events-none fixed z-50 rounded-md bg-foreground px-2 py-1 font-mono text-xs text-background"
            style={{ left: header.x, top: header.y }}
          >
            {header.text}
          </div>
        )}
      </div>
      {result.truncated && (
        <p className="shrink-0 border-t px-3 py-1.5 text-xs text-muted-foreground">
          First {rowLimit.toLocaleString()} rows. The rest weren't fetched.
        </p>
      )}
    </div>
  );
}
