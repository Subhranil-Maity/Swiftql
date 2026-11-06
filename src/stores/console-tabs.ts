import { create } from "zustand";

import {
  type ConsoleTabSet,
  type TabInit,
  createTab,
  emptyTabSet,
  parseTabSet,
} from "@/features/console/model/tab";
import { createDebouncedSave } from "@/lib/debounced-save";
import { api } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

/** Tabs are keyed by saved connection: session ids change on every connect. */
const storageKey = (connectionId: string) => `console.tabs.${connectionId}`;

const SAVE_DELAY_MS = 500;

interface ConsoleTabsState {
  byConnection: Record<string, ConsoleTabSet>;
  /** Reads the saved tabs once per connection; later calls do nothing. */
  load: (connectionId: string) => Promise<void>;
  /** Opens and activates a tab; returns its id, or null while the tabs are still loading. */
  openTab: (connectionId: string, init?: TabInit) => string | null;
  /** Activates the neighbour; closing the last tab leaves a fresh one. */
  closeTab: (connectionId: string, tabId: string) => void;
  selectTab: (connectionId: string, tabId: string) => void;
  /** Moves the active tab by `step`, wrapping around. */
  cycleTab: (connectionId: string, step: 1 | -1) => void;
  setSql: (connectionId: string, tabId: string, sql: string) => void;
}

export const useConsoleTabsStore = create<ConsoleTabsState>()((set, get) => {
  const loading = new Set<string>();

  /** Applies `change` to a loaded tab set and schedules a save. */
  const update = (connectionId: string, change: (tabs: ConsoleTabSet) => ConsoleTabSet) => {
    const current = get().byConnection[connectionId];
    if (!current) return;
    set((s) => ({ byConnection: { ...s.byConnection, [connectionId]: change(current) } }));
    scheduleSave(connectionId);
  };

  // Failures show a toast with Retry; pending saves are flushed when the window closes.
  const { schedule: scheduleSave } = createDebouncedSave("console tabs", SAVE_DELAY_MS, (connectionId) => {
    const tabs = get().byConnection[connectionId];
    return tabs ? api.setAppState(storageKey(connectionId), tabs) : null;
  });

  return {
    byConnection: {},

    load: async (connectionId) => {
      if (get().byConnection[connectionId] || loading.has(connectionId)) return;
      loading.add(connectionId);
      let tabs: ConsoleTabSet | null = null;
      try {
        tabs = parseTabSet(await api.getAppState(storageKey(connectionId)));
      } catch (error) {
        console.warn(`loading console tabs for ${connectionId} failed:`, errorMessage(error));
      } finally {
        loading.delete(connectionId);
      }
      set((s) => ({
        byConnection: { ...s.byConnection, [connectionId]: tabs ?? emptyTabSet() },
      }));
    },

    openTab: (connectionId, init) => {
      if (!get().byConnection[connectionId]) return null;
      let id: string | null = null;
      update(connectionId, (t) => {
        const tab = createTab(t.counter + 1, init);
        id = tab.id;
        return { tabs: [...t.tabs, tab], activeId: tab.id, counter: t.counter + 1 };
      });
      return id;
    },

    closeTab: (connectionId, tabId) =>
      update(connectionId, (t) => {
        const index = t.tabs.findIndex((tab) => tab.id === tabId);
        if (index === -1) return t;
        const tabs = t.tabs.filter((tab) => tab.id !== tabId);
        if (tabs.length === 0) return emptyTabSet();
        const activeId =
          t.activeId === tabId ? tabs[Math.min(index, tabs.length - 1)].id : t.activeId;
        return { ...t, tabs, activeId };
      }),

    selectTab: (connectionId, tabId) =>
      update(connectionId, (t) =>
        t.tabs.some((tab) => tab.id === tabId) ? { ...t, activeId: tabId } : t,
      ),

    cycleTab: (connectionId, step) =>
      update(connectionId, (t) => {
        const index = t.tabs.findIndex((tab) => tab.id === t.activeId);
        const next = (index + step + t.tabs.length) % t.tabs.length;
        return { ...t, activeId: t.tabs[next].id };
      }),

    setSql: (connectionId, tabId, sql) =>
      update(connectionId, (t) => ({
        ...t,
        tabs: t.tabs.map((tab) => (tab.id === tabId ? { ...tab, sql } : tab)),
      })),
  };
});
