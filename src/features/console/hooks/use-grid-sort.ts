import { useCallback, useState } from "react";
import { toast } from "sonner";

import type { SortState } from "@/features/console/model/row-sort";
import { api } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

/**
 * Sorting a held result in the backend, over every held row. Clicking a
 * column cycles ascending → descending → unsorted. `onSorted` runs once the
 * backend has the new order, so cached pages can be dropped.
 */
export function useGridSort(
  sessionId: string,
  resultId: string | undefined,
  loading: boolean,
  onSorted: (sorted: boolean) => void,
) {
  const [sort, setSort] = useState<SortState>(null);
  const [sorting, setSorting] = useState(false);

  const apply = useCallback(
    async (next: SortState) => {
      if (!resultId) return;
      setSorting(true);
      try {
        await api.sortResult(sessionId, resultId, next);
        setSort(next);
        onSorted(next !== null);
      } catch (error) {
        toast.error("Couldn't sort the result", { description: errorMessage(error) });
      } finally {
        setSorting(false);
      }
    },
    [sessionId, resultId, onSorted],
  );

  const cycle = useCallback(
    (column: number) => {
      if (loading) {
        toast.info("Sorting is available once all rows are loaded");
        return;
      }
      if (sorting) return;
      let next: SortState;
      if (sort?.column !== column) next = { column, direction: "asc" };
      else next = sort.direction === "asc" ? { column, direction: "desc" } : null;
      void apply(next);
    },
    [loading, sorting, sort, apply],
  );

  const clear = useCallback(() => void apply(null), [apply]);

  return { sort, sorting, cycle, clear };
}
