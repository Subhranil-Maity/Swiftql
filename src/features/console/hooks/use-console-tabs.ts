import { useEffect, useMemo } from "react";

import { useConsoleTabsStore } from "@/stores/console-tabs";

/** A connection's console tabs, loading them on first use. `tabSet` is undefined while loading. */
export function useConsoleTabs(connectionId: string) {
  const tabSet = useConsoleTabsStore((s) => s.byConnection[connectionId]);
  const load = useConsoleTabsStore((s) => s.load);
  const openTab = useConsoleTabsStore((s) => s.openTab);
  const closeTab = useConsoleTabsStore((s) => s.closeTab);
  const selectTab = useConsoleTabsStore((s) => s.selectTab);
  const cycleTab = useConsoleTabsStore((s) => s.cycleTab);
  const setSql = useConsoleTabsStore((s) => s.setSql);

  useEffect(() => {
    void load(connectionId);
  }, [load, connectionId]);

  const actions = useMemo(
    () => ({
      open: () => openTab(connectionId),
      close: (tabId: string) => closeTab(connectionId, tabId),
      select: (tabId: string) => selectTab(connectionId, tabId),
      cycle: (step: 1 | -1) => cycleTab(connectionId, step),
      setSql: (tabId: string, sql: string) => setSql(connectionId, tabId, sql),
    }),
    [connectionId, openTab, closeTab, selectTab, cycleTab, setSql],
  );

  return { tabSet, actions };
}

export type ConsoleTabActions = ReturnType<typeof useConsoleTabs>["actions"];
