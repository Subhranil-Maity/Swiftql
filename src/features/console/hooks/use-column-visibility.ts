import { useCallback, useMemo, useState } from "react";

/** Which of `count` columns show. At least one always does. */
export function useColumnVisibility(count: number) {
  const [hidden, setHidden] = useState<ReadonlySet<number>>(() => new Set());

  const visible = useMemo(() => {
    const shown: number[] = [];
    for (let i = 0; i < count; i++) if (!hidden.has(i)) shown.push(i);
    return shown;
  }, [count, hidden]);

  const toggle = useCallback(
    (column: number) =>
      setHidden((h) => {
        const next = new Set(h);
        if (next.has(column)) next.delete(column);
        else if (count - next.size > 1) next.add(column);
        return next;
      }),
    [count],
  );

  const showAll = useCallback(() => setHidden(new Set()), []);

  return { hidden, visible, toggle, showAll };
}
