import type { ReactNode } from "react";

import { ColumnType } from "@/components/schema/column-type";
import { KeyGlyph } from "@/components/schema/key-glyph";
import type { ColumnRole } from "@/features/tables/model/table-entry";
import type { Column } from "@/lib/ipc";
import { cn } from "@/lib/utils";

/**
 * Key gutter, name, type and a `?` for nullable columns, on a fixed 24px
 * row. Used by the hover card, the details sheet and the graph's table cards.
 */
export function ColumnRow({
  column,
  role,
  className,
  children,
}: {
  column: Column;
  role: ColumnRole;
  className?: string;
  /** Extra content at the row's ends, e.g. the graph's connection handles. */
  children?: ReactNode;
}) {
  return (
    <div className={cn("relative flex h-6 min-w-0 items-center gap-1.5", className)}>
      <KeyGlyph role={role} />
      <span
        className={cn("min-w-0 truncate", role.foreign && "text-ref")}
        title={column.comment ?? column.name}
      >
        {column.name}
      </span>
      <span className="ml-auto flex min-w-0 shrink items-center gap-0.5 pl-3">
        <ColumnType type={column.dataType} />
        <span
          aria-label={column.nullable ? "nullable" : undefined}
          className="w-2 font-mono text-[11px] text-muted-foreground/70"
        >
          {column.nullable ? "?" : ""}
        </span>
      </span>
      {children}
    </div>
  );
}
