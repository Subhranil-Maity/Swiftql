/** Notes under the grid: rows left unfetched, and what sorting covers then. */
export function ResultFooter({
  truncated,
  rowLimit,
  sorted,
}: {
  truncated: boolean;
  rowLimit: number;
  sorted: boolean;
}) {
  if (!truncated) return null;
  return (
    <p className="shrink-0 border-t px-3 py-1.5 text-xs text-muted-foreground">
      First {rowLimit.toLocaleString()} rows. The rest weren't fetched.
      {sorted && ` Sorting covers the ${rowLimit.toLocaleString()} fetched rows.`}
    </p>
  );
}
