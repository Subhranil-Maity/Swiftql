import "@glideapps/glide-data-grid/dist/index.css";

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  CompactSelection,
  DataEditor,
  type GridCell,
  GridCellKind,
  type GridColumn,
  type GridSelection,
  type Item,
} from "@glideapps/glide-data-grid";
import { toast } from "sonner";

import { CellViewerDialog, type ViewedCell } from "@/features/console/components/cell-viewer-dialog";
import { ResultFooter } from "@/features/console/components/result-footer";
import { ResultToolbar } from "@/features/console/components/result-toolbar";
import { useColumnVisibility } from "@/features/console/hooks/use-column-visibility";
import { useColumnWidths } from "@/features/console/hooks/use-column-widths";
import { useGridSort } from "@/features/console/hooks/use-grid-sort";
import { HEADER_HEIGHT, makeDrawHeader } from "@/features/console/lib/draw-header";
import { gridPalette } from "@/features/console/lib/grid-theme";
import { formatCell } from "@/features/console/model/cell-format";
import {
  type ExportFormat,
  type ExportSlice,
  FORMAT_EXTENSIONS,
  FORMAT_LABELS,
  serialize,
} from "@/features/console/model/result-export";
import type { StatementResult } from "@/features/console/model/run";
import { plural } from "@/lib/format-count";
import type { DbValue } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { saveFile } from "@/lib/save-file";
import { useIsDark } from "@/stores/theme";

const NULL_VALUE: DbValue = { type: "null" };

const EMPTY_SELECTION: GridSelection = {
  columns: CompactSelection.empty(),
  rows: CompactSelection.empty(),
};

/** Fonts are drawn on canvas, so the grid redraws once they've loaded. */
function useFontsReady(): boolean {
  const [ready, setReady] = useState(document.fonts.status === "loaded");
  useEffect(() => {
    if (!ready) void document.fonts.ready.then(() => setReady(true));
  }, [ready]);
  return ready;
}

/** Display rows and source columns the selection covers; null when it's a single cell or nothing. */
function selectionCells(
  selection: GridSelection,
  visible: number[],
  rowCount: number,
): { rows: number[]; columns: number[] } | null {
  const all = (n: number) => Array.from({ length: n }, (_, i) => i);
  if (selection.rows.length > 0) return { rows: selection.rows.toArray(), columns: visible };
  if (selection.columns.length > 0) {
    return { rows: all(rowCount), columns: selection.columns.toArray().map((c) => visible[c]) };
  }
  const range = selection.current?.range;
  if (!range || range.width * range.height <= 1) return null;
  return {
    rows: all(range.height).map((i) => range.y + i),
    columns: visible.slice(range.x, range.x + range.width),
  };
}

/** One statement's rows: read-only, virtualized, sortable, copyable with Ctrl+C, searchable with Ctrl+F. */
export function ResultGrid({ result, rowLimit }: { result: StatementResult; rowLimit: number }) {
  const isDark = useIsDark();
  const fontsReady = useFontsReady();
  const containerRef = useRef<HTMLDivElement>(null);
  // Read where --tab-accent (the connection colour) is set, again when the theme or fonts change.
  const [palette, setPalette] = useState(() => gridPalette());
  useLayoutEffect(() => {
    setPalette(gridPalette(containerRef.current ?? undefined));
  }, [isDark, fontsReady]);
  const theme = palette.theme;

  const columns = useMemo(() => result.columns ?? [], [result.columns]);
  const rows = result.rows;
  const hasRows = rows.length > 0;

  const { sort, order, cycle, clear } = useGridSort(rows);
  const { hidden, visible, toggle, showAll } = useColumnVisibility(columns.length);
  const { widthOf, resize } = useColumnWidths(columns, rows, fontsReady);
  const [selection, setSelection] = useState<GridSelection>(EMPTY_SELECTION);
  const [viewed, setViewed] = useState<ViewedCell | null>(null);
  // The selection is in display positions, which sorting and hiding columns reshuffle.
  useEffect(() => setSelection(EMPTY_SELECTION), [visible, sort]);

  /** The fetched row shown at display position `row`. */
  const rowAt = useCallback((row: number) => rows[order ? order[row] : row], [rows, order]);

  const gridColumns = useMemo<GridColumn[]>(
    () =>
      visible.map((i) => ({
        id: String(i),
        title: columns[i].name,
        width: widthOf(i),
      })),
    [visible, columns, widthOf],
  );

  const drawHeader = useMemo(
    () => makeDrawHeader(columns.map((c) => c.nativeType), sort, palette),
    [columns, sort, palette],
  );

  const getCellContent = useCallback(
    ([col, row]: Item): GridCell => {
      const cell = formatCell(rowAt(row)?.[visible[col]] ?? NULL_VALUE);
      return {
        kind: GridCellKind.Text,
        data: cell.copy,
        displayData: cell.display,
        copyData: cell.copy,
        // The cell viewer (double-click / Enter) shows long values instead.
        allowOverlay: false,
        readonly: true,
        contentAlign: cell.kind === "number" ? "right" : "left",
        themeOverride:
          cell.kind === "null"
            ? { textDark: theme.textLight, baseFontStyle: "italic 12.5px" }
            : undefined,
      };
    },
    [rowAt, visible, theme],
  );

  const getRowThemeOverride = useCallback(
    (row: number) => (row % 2 === 1 ? { bgCell: palette.zebra } : undefined),
    [palette],
  );

  /** The selection (or, without one, every row) in display order, visible columns only. */
  const slice = (useSelection: boolean): ExportSlice => {
    const cells = useSelection ? selectionCells(selection, visible, rows.length) : null;
    const rowIndices = cells?.rows ?? Array.from({ length: rows.length }, (_, i) => i);
    const columnIndices = cells?.columns ?? visible;
    return {
      columns: columnIndices.map((i) => columns[i]),
      rows: rowIndices.map((r) => columnIndices.map((c) => rowAt(r)?.[c] ?? NULL_VALUE)),
    };
  };
  const hasSelection = selectionCells(selection, visible, rows.length) !== null;

  const copyAs = async (format: ExportFormat) => {
    const part = slice(true);
    try {
      await navigator.clipboard.writeText(serialize(part, format));
      toast.success(`Copied ${plural(part.rows.length, "row")} as ${FORMAT_LABELS[format]}`);
    } catch {
      toast.error("Couldn't copy to the clipboard");
    }
  };

  const exportAs = async (format: ExportFormat) => {
    const ext = FORMAT_EXTENSIONS[format];
    try {
      const path = await saveFile(`result.${ext}`, serialize(slice(false), format), [
        { name: FORMAT_LABELS[format], extensions: [ext] },
      ]);
      if (path) toast.success(`Saved ${plural(rows.length, "row")}`, { description: path });
    } catch (error) {
      toast.error("Couldn't save the export", { description: errorMessage(error) });
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <ResultToolbar
        rowCount={rows.length}
        columns={columns}
        sort={sort}
        onClearSort={clear}
        hidden={hidden}
        onToggleColumn={toggle}
        onShowAllColumns={showAll}
        hasSelection={hasSelection}
        onCopy={(format) => void copyAs(format)}
        onExport={(format) => void exportAs(format)}
      />
      <div ref={containerRef} className="relative min-h-0 flex-1">
        <DataEditor
          columns={gridColumns}
          rows={rows.length}
          getCellContent={getCellContent}
          getCellsForSelection
          getRowThemeOverride={getRowThemeOverride}
          drawHeader={drawHeader}
          theme={theme}
          width="100%"
          height="100%"
          rowHeight={26}
          headerHeight={HEADER_HEIGHT}
          rowMarkers="number"
          smoothScrollX
          smoothScrollY
          keybindings={{ search: true }}
          gridSelection={selection}
          onGridSelectionChange={setSelection}
          cellActivationBehavior="double-click"
          onCellActivated={([col, row]) => {
            const column = columns[visible[col]];
            if (column) setViewed({ column, value: rowAt(row)?.[visible[col]] ?? NULL_VALUE, row });
          }}
          onHeaderClicked={(col) => {
            if (visible[col] !== undefined) cycle(visible[col]);
          }}
          onColumnResize={(column, size) => resize(Number(column.id), size)}
        />
        {!hasRows && result.status !== "running" && (
          <p className="pointer-events-none absolute inset-x-0 top-16 text-center text-muted-foreground">
            No rows
          </p>
        )}
      </div>
      <ResultFooter truncated={result.truncated} rowLimit={rowLimit} sorted={sort !== null} />
      <CellViewerDialog cell={viewed} onClose={() => setViewed(null)} />
    </div>
  );
}
