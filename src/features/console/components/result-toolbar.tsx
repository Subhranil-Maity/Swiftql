import { XIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ColumnsMenu } from "@/features/console/components/columns-menu";
import { CopyAsMenu } from "@/features/console/components/copy-as-menu";
import { ExportMenu } from "@/features/console/components/export-menu";
import type { SortState } from "@/features/console/model/row-sort";
import type { ExportFormat } from "@/features/console/model/result-export";
import { plural } from "@/lib/format-count";
import type { ColumnMeta } from "@/lib/ipc";

/** Above the grid: row count and sort on the left; columns, copy and export on the right. */
export function ResultToolbar({
  rowCount,
  loading,
  columns,
  sort,
  sorting,
  onClearSort,
  hidden,
  onToggleColumn,
  onShowAllColumns,
  hasSelection,
  onCopy,
  onExport,
}: {
  rowCount: number;
  /** Rows are still read; the count grows. */
  loading: boolean;
  columns: ColumnMeta[];
  sort: SortState;
  /** The backend is sorting. */
  sorting: boolean;
  onClearSort: () => void;
  hidden: ReadonlySet<number>;
  onToggleColumn: (column: number) => void;
  onShowAllColumns: () => void;
  hasSelection: boolean;
  onCopy: (format: ExportFormat) => void;
  onExport: (format: ExportFormat) => void;
}) {
  const sortedBy = sort ? columns[sort.column]?.name : undefined;
  return (
    <div className="flex h-8 shrink-0 items-center gap-1 border-b px-2 text-xs">
      <span className="px-1 text-muted-foreground tabular-nums">{plural(rowCount, "row")}
        {loading && "…"}
      </span>
      {sorting && <span className="text-muted-foreground">· sorting…</span>}
      {sort && sortedBy && (
        <span className="flex items-center text-muted-foreground">
          · sorted by <span className="ml-1 font-medium text-foreground">{sortedBy}</span>
          <span className="ml-1">{sort.direction === "asc" ? "↑" : "↓"}</span>
          <Button variant="ghost" size="icon-xs" aria-label="Clear sort" className="ml-0.5" onClick={onClearSort}>
            <XIcon />
          </Button>
        </span>
      )}
      <div className="ml-auto flex items-center gap-0.5">
        <ColumnsMenu
          columns={columns}
          hidden={hidden}
          onToggle={onToggleColumn}
          onShowAll={onShowAllColumns}
        />
        <CopyAsMenu hasSelection={hasSelection} onCopy={onCopy} />
        <ExportMenu onExport={onExport} />
      </div>
    </div>
  );
}
