import { ColumnRow } from "@/components/schema/column-row";
import { TableKindIcon, tableKindLabel } from "@/components/schema/table-kind-icon";
import { TableStats } from "@/components/schema/table-stats";
import { HoverCard, HoverCardContent, createHoverCardHandle } from "@/components/ui/hover-card";
import { columnRole, type TableEntry } from "@/features/tables/model/table-entry";

/** Columns shown before "+N more". */
const MAX_COLUMNS = 15;

/** One card shared by every row of a list; rows pass their table key as the payload. */
export const tableHoverCard = createHoverCardHandle<string>();

/** The card itself: name, kind, comment, size and the first columns. */
export function TableSummary({ entry }: { entry: TableEntry }) {
  const { table } = entry;
  const columns = table.columns ?? [];
  const shown = columns.slice(0, MAX_COLUMNS);
  return (
    <div className="flex flex-col gap-2">
      <div className="flex min-w-0 items-center gap-2">
        <TableKindIcon kind={table.kind} />
        <span className="min-w-0 truncate font-medium">{table.name}</span>
        <span className="ml-auto shrink-0 text-xs text-muted-foreground">
          {tableKindLabel(table.kind)}
        </span>
      </div>
      {table.comment && <p className="text-xs text-muted-foreground">{table.comment}</p>}
      <TableStats table={table} />
      {columns.length > 0 && (
        <div className="border-t pt-1.5">
          {shown.map((column) => (
            <ColumnRow key={column.name} column={column} role={columnRole(table, column)} />
          ))}
          {columns.length > shown.length && (
            <p className="pt-1 pl-5 text-xs text-muted-foreground">
              +{columns.length - shown.length} more
            </p>
          )}
        </div>
      )}
    </div>
  );
}

/** Mount once per list; `find` turns the hovered row's key back into its table. */
export function TableHoverCard({ find }: { find: (key: string) => TableEntry | undefined }) {
  return (
    <HoverCard handle={tableHoverCard}>
      {({ payload }) => {
        const entry = payload ? find(payload as string) : undefined;
        return (
          entry && (
            <HoverCardContent side="right" align="start" sideOffset={12} className="w-80">
              <TableSummary entry={entry} />
            </HoverCardContent>
          )
        );
      }}
    </HoverCard>
  );
}
