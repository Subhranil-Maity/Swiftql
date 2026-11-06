import { TableKindIcon, tableKindLabel } from "@/components/schema/table-kind-icon";
import { HoverCardTrigger } from "@/components/ui/hover-card";
import { tableHoverCard } from "@/features/tables/components/table-hover-card";
import type { TableEntry } from "@/features/tables/model/table-entry";
import { formatEstimate } from "@/lib/format-count";

/** One table row (28px): kind icon, name, and the row estimate on the right. */
export function TableListItem({
  entry,
  onOpen,
  treeProps,
}: {
  entry: TableEntry;
  onOpen: () => void;
  /** Roving tab stop from `useListKeyboard`. */
  treeProps: { "data-tree-id": string; tabIndex: number; onFocus: () => void };
}) {
  const { table } = entry;
  return (
    <HoverCardTrigger
      handle={tableHoverCard}
      payload={entry.key}
      delay={450}
      closeDelay={100}
      render={
        <div
          role="treeitem"
          aria-level={2}
          data-table-key={entry.key}
          {...treeProps}
          onDoubleClick={onOpen}
          className="flex h-7 cursor-default items-center gap-2 rounded-md pr-2 pl-7 outline-none select-none hover:bg-muted/60 focus-visible:bg-muted focus-visible:shadow-[inset_2px_0_0_var(--tab-accent)]"
        />
      }
    >
      <TableKindIcon kind={table.kind} />
      <span className="min-w-0 truncate">{table.name}</span>
      {table.kind !== "table" && (
        <span className="shrink-0 text-xs text-muted-foreground">{tableKindLabel(table.kind)}</span>
      )}
      {table.rowEstimate != null && (
        <span className="ml-auto shrink-0 pl-4 text-xs text-muted-foreground tabular-nums">
          {formatEstimate(table.rowEstimate)}
        </span>
      )}
    </HoverCardTrigger>
  );
}
