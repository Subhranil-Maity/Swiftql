import { Spinner } from "@/components/ui/spinner";
import type { ResultState } from "@/lib/ipc";

/** Under the grid: loading progress, or why reading stopped before the end. */
export function ResultFooter({
  state,
  total,
  error,
}: {
  /** Undefined while rows are still read. */
  state: ResultState | undefined;
  total: number;
  error: string | null;
}) {
  const count = total.toLocaleString();
  let note: React.ReactNode = null;
  if (error) {
    note = <span className="text-destructive">Couldn't load rows: {error}</span>;
  } else if (state === undefined || state === "loading") {
    note = (
      <span className="flex items-center gap-1.5">
        <Spinner className="size-3" />
        Loading… {count} rows
      </span>
    );
  } else if (state === "truncated") {
    note = `${count} rows: the memory limit was reached and the rest weren't read. Raise it in Settings → Query & results.`;
  } else if (state === "cancelled") {
    note = `Stopped after ${count} rows.`;
  }
  if (!note) return null;
  return <div className="shrink-0 border-t px-3 py-1.5 text-xs text-muted-foreground">{note}</div>;
}
