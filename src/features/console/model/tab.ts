// Console tabs: one scratch buffer of SQL each. A connection's tab set is
// saved to app state, so it must survive a round-trip through JSON.

export type ConsoleTab = {
  id: string;
  title: string;
  sql: string;
};

export type ConsoleTabSet = {
  tabs: ConsoleTab[];
  activeId: string;
  /** Number of the last "Query N" handed out; only ever grows. */
  counter: number;
};

export function createTab(n: number): ConsoleTab {
  return { id: crypto.randomUUID(), title: `Query ${n}`, sql: "" };
}

export function emptyTabSet(): ConsoleTabSet {
  const tab = createTab(1);
  return { tabs: [tab], activeId: tab.id, counter: 1 };
}

function isTab(value: unknown): value is ConsoleTab {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.id === "string" && typeof v.title === "string" && typeof v.sql === "string";
}

/** Reads a saved tab set; null when the value is missing or malformed. */
export function parseTabSet(value: unknown): ConsoleTabSet | null {
  if (typeof value !== "object" || value === null) return null;
  const v = value as Record<string, unknown>;
  if (!Array.isArray(v.tabs) || v.tabs.length === 0 || !v.tabs.every(isTab)) return null;
  const tabs = v.tabs as ConsoleTab[];
  const counter = typeof v.counter === "number" ? v.counter : tabs.length;
  // A stale active id falls back to the first tab instead of failing the load.
  const activeId = tabs.some((t) => t.id === v.activeId) ? (v.activeId as string) : tabs[0].id;
  return { tabs, activeId, counter };
}
