import { useCallback, useEffect, useRef, useState } from "react";

import type { StatementResult } from "@/features/console/model/run";
import { api, type DbValue } from "@/lib/ipc";
import { useSettingsStore } from "@/stores/settings";

const DEFAULT_PAGE_SIZE = 1000;

/** Pages fetched past the visible ones, so scrolling rarely shows loading cells. */
const PREFETCH_PAGES = 1;

/** Cached pages beyond this are dropped, farthest from the view first. */
const MAX_CACHED_PAGES = 40;

type Region = { y: number; height: number };

/**
 * Rows of a result held in the backend, fetched a page at a time as the grid
 * scrolls. `rowAt` returns undefined for rows not fetched yet; the grid shows
 * a loading cell and redraws when `version` changes.
 *
 * `reset()` drops the cache, e.g. after the result was sorted.
 */
export function useResultPages(sessionId: string, result: StatementResult) {
  const pageSize = useSettingsStore((s) => s.settings?.query.pageSize ?? DEFAULT_PAGE_SIZE);
  const resultId = result.resultId;
  const total = result.total;
  const loading = result.resultState === undefined;

  const pages = useRef(new Map<number, DbValue[][]>());
  const pending = useRef(new Set<number>());
  /** Bumped by `reset`, so answers to requests made before it are ignored. */
  const generation = useRef(0);
  const region = useRef<Region>({ y: 0, height: 50 });
  const sorted = useRef(false);
  const [version, setVersion] = useState(0);
  const [error, setError] = useState<string | null>(null);

  // Page 0 arrives with the run itself.
  if (!sorted.current && !pages.current.has(0) && result.rows.length > 0) {
    pages.current.set(0, result.rows.slice(0, pageSize));
  }

  const fetchPage = useCallback(
    async (page: number) => {
      if (!resultId || pending.current.has(page)) return;
      pending.current.add(page);
      const gen = generation.current;
      try {
        const got = await api.fetchRows(sessionId, resultId, page * pageSize, pageSize);
        if (gen !== generation.current) return;
        pages.current.set(page, got.rows);
        trim(pages.current, page);
        setError(null);
        setVersion((v) => v + 1);
      } catch (err) {
        if (gen === generation.current) setError(err instanceof Error ? err.message : String(err));
      } finally {
        pending.current.delete(page);
      }
    },
    [sessionId, resultId, pageSize],
  );

  /** Fetches the pages around the visible rows that are missing or still filling up. */
  const ensureVisible = useCallback(() => {
    const { y, height } = region.current;
    const first = Math.floor(y / pageSize);
    const last = Math.floor((y + height) / pageSize) + PREFETCH_PAGES;
    for (let page = first; page <= last; page++) {
      const start = page * pageSize;
      if (start >= total) break;
      const cached = pages.current.get(page);
      const expected = Math.min(pageSize, total - start);
      if (!cached || cached.length < expected) void fetchPage(page);
    }
  }, [pageSize, total, fetchPage]);

  // More rows arrived (loading) or the cache was reset: fill what's on screen.
  useEffect(() => {
    ensureVisible();
  }, [ensureVisible, version, loading]);

  const onVisibleRegionChanged = useCallback(
    (r: { y: number; height: number }) => {
      region.current = { y: r.y, height: r.height };
      ensureVisible();
    },
    [ensureVisible],
  );

  const rowAt = useCallback(
    (row: number): DbValue[] | undefined =>
      pages.current.get(Math.floor(row / pageSize))?.[row % pageSize],
    // version: the cache is a ref; new pages must give a new function so the grid redraws.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [pageSize, version],
  );

  const reset = useCallback((isSorted: boolean) => {
    generation.current++;
    sorted.current = isSorted;
    pages.current = new Map();
    pending.current = new Set();
    setVersion((v) => v + 1);
  }, []);

  return { rowAt, total, loading, error, onVisibleRegionChanged, reset, version };
}

/** Drops the pages farthest from `near` once there are too many. */
function trim(pages: Map<number, DbValue[][]>, near: number) {
  if (pages.size <= MAX_CACHED_PAGES) return;
  const far = [...pages.keys()].sort((a, b) => Math.abs(b - near) - Math.abs(a - near));
  for (const page of far.slice(0, pages.size - MAX_CACHED_PAGES)) pages.delete(page);
}

/** Row indices as `[start, end)` ranges, for sending a selection to the backend. */
export function toRanges(rows: number[]): [number, number][] {
  const sorted = [...rows].sort((a, b) => a - b);
  const ranges: [number, number][] = [];
  for (const r of sorted) {
    const last = ranges[ranges.length - 1];
    if (last && last[1] === r) last[1] = r + 1;
    else if (!last || last[1] < r) ranges.push([r, r + 1]);
  }
  return ranges;
}

/** Rows `[start, end)` of a result in display order, fetched in pages. */
export async function fetchRange(
  sessionId: string,
  resultId: string,
  start: number,
  end: number,
): Promise<DbValue[][]> {
  const out: DbValue[][] = [];
  const CHUNK = 10_000;
  for (let offset = start; offset < end; offset += CHUNK) {
    const page = await api.fetchRows(sessionId, resultId, offset, Math.min(CHUNK, end - offset));
    out.push(...page.rows);
    if (page.rows.length === 0) break;
  }
  return out;
}
