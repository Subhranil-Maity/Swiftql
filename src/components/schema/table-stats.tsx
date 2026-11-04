import { formatBytes } from "@/lib/format-bytes";
import { formatEstimate } from "@/lib/format-count";
import type { Table } from "@/lib/ipc";
import { cn } from "@/lib/utils";

/** "~340k rows · 48 MB"; parts the server didn't report are left out. */
export function TableStats({ table, className }: { table: Table; className?: string }) {
  const parts: string[] = [];
  if (table.rowEstimate != null) parts.push(`${formatEstimate(table.rowEstimate)} rows`);
  if (table.sizeBytes != null) parts.push(formatBytes(table.sizeBytes));
  if (parts.length === 0) return null;
  return (
    <p
      className={cn("text-xs text-muted-foreground tabular-nums", className)}
      title="Row count is the planner's estimate"
    >
      {parts.join(" · ")}
    </p>
  );
}
