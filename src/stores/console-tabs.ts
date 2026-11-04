import { create } from "zustand";

import {
  type ConsoleTabSet,
  createTab,
  emptyTabSet,
  parseTabSet,
} from "@/features/console/model/tab";
import { api } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

/** Tabs are keyed by saved connection: session ids change on every connect. */
const storageKey = (connectionId: string) => `console.tabs.${connectionId}`;

const SAVE_DELAY_MS = 500;
const pendingSaves = new Map<string, ReturnType<typeof setTimeout>>();

interface ConsoleTabsState {
  byConnection: Record<string, ConsoleTabSet>;
  /** Reads the saved tabs once per connection; later calls do nothing. */
  load: (connectionId: string) => Promise<void>;
  openTab: (connectionId: string) => void;
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

  const scheduleSave = (connectionId: string) => {
    clearTimeout(pendingSaves.get(connectionId));
    pendingSaves.set(
      connectionId,
      setTimeout(() => {
        pendingSaves.delete(connectionId);
        const tabs = get().byConnection[connectionId];
        if (!tabs) return;
        // TODO: surface save failures in the UI instead of only logging them
        api.setAppState(storageKey(connectionId), tabs).catch((error: unknown) => {
          console.warn(`saving console tabs for ${connectionId} failed:`, errorMessage(error));
        });
      }, SAVE_DELAY_MS),
    );
    // TODO: flush pending saves when the window closes; the last ~500ms of typing is lost on quit
  };

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

    openTab: (connectionId) =>
      update(connectionId, (t) => {
        const tab = createTab(t.counter + 1);
        return { tabs: [...t.tabs, tab], activeId: tab.id, counter: t.counter + 1 };
      }),

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
