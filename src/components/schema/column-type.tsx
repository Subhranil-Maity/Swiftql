import { cn } from "@/lib/utils";

/** A column's SQL type: muted 11px mono, since it is literally code. */
export function ColumnType({ type, className }: { type: string; className?: string }) {
  return (
    <span className={cn("truncate font-mono text-[11px] text-muted-foreground", className)} title={type}>
      {type}
    </span>
  );
}
