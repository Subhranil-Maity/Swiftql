import { memo } from "react";
import type { NodeProps } from "@xyflow/react";

import { ColumnRow } from "@/components/schema/column-row";
import { TableKindIcon } from "@/components/schema/table-kind-icon";
import { RowHandles } from "@/features/graph/components/column-handle";
import { cardHandle, leftHandle, rightHandle, type TableNode } from "@/features/graph/model/build-graph";
import { useGraphHighlightState } from "@/features/graph/model/highlight-context";
import { BODY_PADDING_Y, HEADER_HEIGHT, nodeColumns, ROW_HEIGHT } from "@/features/graph/model/node-size";
import { columnRole } from "@/features/tables/model/table-entry";
import { cn } from "@/lib/utils";

/**
 * A table as a drafting card: a header tinted with the connection colour,
 * then one 24px row per column with the key glyphs in a fixed gutter.
 * Sizes follow node-size.ts, which the layout is computed from.
 */
export const TableNodeView = memo(function TableNodeView({ id, data, selected, dragging }: NodeProps<TableNode>) {
  const highlight = useGraphHighlightState();
  const dimmed = highlight.nodes !== null && !highlight.nodes.has(id);
  const { table } = data.entry;
  const { shown, hidden } = nodeColumns(table);

  return (
    <div
      style={{ width: data.size.width }}
      className={cn(
        "overflow-hidden rounded-md border bg-background text-xs text-foreground transition-[opacity,box-shadow,border-color] duration-150 motion-reduce:transition-none",
        (selected || dragging) && "border-(--tab-accent) shadow-md",
        dimmed && "opacity-35",
      )}
    >
      <div
        className="relative flex items-center gap-1.5 border-b px-2.5"
        style={{
          height: HEADER_HEIGHT,
          background: "color-mix(in oklch, var(--tab-accent) 13%, var(--background))",
        }}
      >
        <TableKindIcon kind={table.kind} className="text-foreground/70" />
        <span className="min-w-0 truncate text-[12.5px] font-medium">{table.name}</span>
        {data.schemaLabel && (
          <span className="ml-auto shrink-0 pl-2 font-mono text-[11px] text-muted-foreground">
            {data.schemaLabel}
          </span>
        )}
        {/* For edges to a table whose columns aren't listed. */}
        <RowHandles left={cardHandle("l")} right={cardHandle("r")} />
      </div>
      <div style={{ paddingBlock: BODY_PADDING_Y }}>
        {shown.map((column) => (
          <ColumnRow
            key={column.name}
            column={column}
            role={columnRole(table, column)}
            className="px-2.5"
          >
            <RowHandles left={leftHandle(column.name)} right={rightHandle(column.name)} />
          </ColumnRow>
        ))}
        {hidden > 0 && (
          <p className="flex items-center pl-7 text-muted-foreground" style={{ height: ROW_HEIGHT }}>
            +{hidden} more columns
          </p>
        )}
      </div>
    </div>
  );
});
