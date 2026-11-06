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
  type Rectangle,
} from "@glideapps/glide-data-grid";
import { toast } from "sonner";

import { CellViewerDialog, type ViewedCell } from "@/features/console/components/cell-viewer-dialog";
import { ResultFooter } from "@/features/console/components/result-footer";
import { ResultToolbar } from "@/features/console/components/result-toolbar";
import { useColumnVisibility } from "@/features/console/hooks/use-column-visibility";
import { useColumnWidths } from "@/features/console/hooks/use-column-widths";
import { useGridSort } from "@/features/console/hooks/use-grid-sort";
import { fetchRange, toRanges, useResultPages } from "@/features/console/hooks/use-result-pages";
import { HEADER_HEIGHT, makeDrawHeader } from "@/features/console/lib/draw-header";
import { gridPalette } from "@/features/console/lib/grid-theme";
import { useCellFormatter } from "@/features/console/model/cell-format";
import { type ExportFormat, FORMAT_EXTENSIONS, FORMAT_LABELS } from "@/features/console/model/result-export";
import type { StatementResult } from "@/features/console/model/run";
import { plural } from "@/lib/format-count";
import { api, type DbValue, type ResultSelection } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { useIsDark } from "@/stores/theme";

const NULL_VALUE: DbValue = { type: "null" };

/** Most rows Ctrl+C copies through the grid; Copy as handles more. */
const MAX_GRID_COPY_ROWS = 100_000;

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

/** The part of the result a selection covers, in display rows and source columns; null for a single cell or nothing. */
function selectionOf(selection: GridSelection, visible: number[]): ResultSelection | null {
  if (selection.rows.length > 0) return { rows: toRanges(selection.rows.toArray()), columns: visible };
  if (selection.columns.length > 0) {
    return { rows: null, columns: selection.columns.toArray().map((c) => visible[c]) };
  }
  const range = selection.current?.range;
  if (!range || range.width * range.height <= 1) return null;
  return {
    rows: [[range.y, range.y + range.height]],
    columns: visible.slice(range.x, range.x + range.width),
  };
}

/**
 * One statement's rows: read-only, virtualized, paged from the backend as it
 * scrolls, sortable over the whole result, copyable with Ctrl+C, searchable
 * with Ctrl+F (over the fetched rows).
 */
export function ResultGrid({ result, sessionId }: { result: StatementResult; sessionId: string }) {
  const isDark = useIsDark();
  const fontsReady = useFontsReady();
  const formatCell = useCellFormatter();
  const containerRef = useRef<HTMLDivElement>(null);
  // Read where --tab-accent (the connection colour) is set, again when the theme or fonts change.
  const [palette, setPalette] = useState(() => gridPalette());
  useLayoutEffect(() => {
    setPalette(gridPalette(containerRef.current ?? undefined));
  }, [isDark, fontsReady]);
  const theme = palette.theme;

  const columns = useMemo(() => result.columns ?? [], [result.columns]);
  const pages = useResultPages(sessionId, result);
  const { rowAt, total, loading } = pages;

  const { sort, sorting, cycle, clear } = useGridSort(sessionId, result.resultId, loading, pages.reset);
  const { hidden, visible, toggle, showAll } = useColumnVisibility(columns.length);
  const { widthOf, resize } = useColumnWidths(columns, result.rows, fontsReady);
  const [selection, setSelection] = useState<GridSelection>(EMPTY_SELECTION);
  const [viewed, setViewed] = useState<ViewedCell | null>(null);
  // The selection is in display positions, which sorting and hiding columns reshuffle.
  useEffect(() => setSelection(EMPTY_SELECTION), [visible, sort]);

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

  const cellFor = useCallback(
    (value: DbValue): GridCell => {
      const cell = formatCell(value);
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
          cell.kind === "null" ? { textDark: theme.textLight, baseFontStyle: "italic 12.5px" } : undefined,
      };
    },
    [formatCell, theme],
  );

  const getCellContent = useCallback(
    ([col, row]: Item): GridCell => {
      const values = rowAt(row);
      if (!values) return { kind: GridCellKind.Loading, allowOverlay: false };
      return cellFor(values[visible[col]] ?? NULL_VALUE);
    },
    [rowAt, visible, cellFor],
  );

  /** Ctrl+C over a range: the rows may not be fetched yet, so ask the backend. */
  const getCellsForSelection = useCallback(
    (rect: Rectangle) => async () => {
      if (!result.resultId) return [];
      const height = Math.min(rect.height, MAX_GRID_COPY_ROWS);
      if (height < rect.height) {
        toast.info(`Copied the first ${MAX_GRID_COPY_ROWS.toLocaleString()} rows; use Copy as for more`);
      }
      const rows = await fetchRange(sessionId, result.resultId, rect.y, rect.y + height);
      const cols = visible.slice(rect.x, rect.x + rect.width);
      return rows.map((values) => cols.map((c) => cellFor(values[c] ?? NULL_VALUE)));
    },
    [sessionId, result.resultId, visible, cellFor],
  );

  const getRowThemeOverride = useCallback(
    (row: number) => (row % 2 === 1 ? { bgCell: palette.zebra } : undefined),
    [palette],
  );

  const part = selectionOf(selection, visible);
  const whole: ResultSelection = { rows: null, columns: visible };

  const copyAs = async (format: ExportFormat) => {
    if (!result.resultId) return;
    const target = part ?? whole;
    try {
      const text = await api.copyResult(sessionId, result.resultId, format, target);
      await navigator.clipboard.writeText(text);
      const count = target.rows ? target.rows.reduce((n, [s, e]) => n + (e - s), 0) : total;
      toast.success(`Copied ${plural(count, "row")} as ${FORMAT_LABELS[format]}`);
    } catch (error) {
      toast.error("Couldn't copy", { description: errorMessage(error) });
    }
  };

  const exportAs = async (format: ExportFormat) => {
    if (!result.resultId) return;
    try {
      const outcome = await api.exportResult(
        sessionId,
        result.resultId,
        format,
        whole,
        `result.${FORMAT_EXTENSIONS[format]}`,
      );
      if (outcome) toast.success(`Saved ${plural(outcome.rows, "row")}`, { description: outcome.path });
    } catch (error) {
      toast.error("Couldn't save the export", { description: errorMessage(error) });
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <ResultToolbar
        rowCount={total}
        loading={loading}
        columns={columns}
        sort={sort}
        sorting={sorting}
        onClearSort={clear}
        hidden={hidden}
        onToggleColumn={toggle}
        onShowAllColumns={showAll}
        hasSelection={part !== null}
        onCopy={(format) => void copyAs(format)}
        onExport={(format) => void exportAs(format)}
      />
      <div ref={containerRef} className="relative min-h-0 flex-1">
        <DataEditor
          columns={gridColumns}
          rows={total}
          getCellContent={getCellContent}
          getCellsForSelection={getCellsForSelection}
          onVisibleRegionChanged={pages.onVisibleRegionChanged}
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
            const values = rowAt(row);
            if (column && values) setViewed({ column, value: values[visible[col]] ?? NULL_VALUE, row });
          }}
          onHeaderClicked={(col) => {
            if (visible[col] !== undefined) cycle(visible[col]);
          }}
          onColumnResize={(column, size) => resize(Number(column.id), size)}
        />
        {total === 0 && result.status !== "running" && (
          <p className="pointer-events-none absolute inset-x-0 top-16 text-center text-muted-foreground">
            No rows
          </p>
        )}
      </div>
      <ResultFooter state={result.resultState} total={total} error={pages.error} />
      <CellViewerDialog cell={viewed} onClose={() => setViewed(null)} />
    </div>
  );
}
