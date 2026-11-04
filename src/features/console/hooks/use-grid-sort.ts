import { useCallback, useMemo, useState } from "react";

import { type SortState, sortRows } from "@/features/console/model/row-sort";
import type { DbValue } from "@/lib/ipc";

/**
 * Client-side sort of the fetched rows. Clicking a column cycles
 * ascending → descending → unsorted. `order` maps display row → fetched row;
 * null while unsorted.
 */
export function useGridSort(rows: DbValue[][]) {
  const [sort, setSort] = useState<SortState>(null);

  const cycle = useCallback(
    (column: number) =>
      setSort((s) => {
        if (s?.column !== column) return { column, direction: "asc" };
        return s.direction === "asc" ? { column, direction: "desc" } : null;
      }),
    [],
  );

  // Rows still streaming in re-sort with each batch.
  const order = useMemo(
    () => (sort ? sortRows(rows, sort.column, sort.direction) : null),
    [rows, sort],
  );

  return { sort, order, cycle, clear: useCallback(() => setSort(null), []) };
}
