import { type CSSProperties, useEffect, useState } from "react";

import { Spinner } from "@/components/ui/spinner";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { CloseTabDialog } from "@/features/console/components/close-tab-dialog";
import { ConsolePane } from "@/features/console/components/console-pane";
import { ConsoleTabBar } from "@/features/console/components/console-tab-bar";
import { RunButton } from "@/features/console/components/run-button";
import { useConsoleShortcuts } from "@/features/console/hooks/use-console-shortcuts";
import { useConsoleTabs } from "@/features/console/hooks/use-console-tabs";
import { cancelRun, useRunQuery } from "@/features/console/hooks/use-run-query";
import { colorVar } from "@/features/connections/model/colors";
import { useQueryRunsStore } from "@/stores/query-runs";
import { useSchemaCatalogStore } from "@/stores/schema-catalog";

/** The tabbed SQL console for one connection. */
export function ConsoleView({
  connectionId,
  sessionId,
  driverId,
  color,
}: {
  connectionId: string;
  /** The open session queries run on. */
  sessionId: string;
  /** Session's driver; picks the editor's SQL dialect and syntax checker. */
  driverId: string;
  /** Connection colour; marks the active tab. */
  color: string | null | undefined;
}) {
  const { tabSet, actions } = useConsoleTabs(connectionId);
  /** Tab waiting for the user to confirm it may be closed. */
  const [confirmingId, setConfirmingId] = useState<string | null>(null);
  const confirming = tabSet?.tabs.find((t) => t.id === confirmingId) ?? null;
  const activeRun = useRunQuery(tabSet?.activeId ?? "", sessionId, driverId);

  // Tables and columns for autocomplete.
  const loadCatalog = useSchemaCatalogStore((s) => s.load);
  useEffect(() => {
    void loadCatalog(sessionId);
  }, [loadCatalog, sessionId]);

  /** Closes a tab, stopping its run and dropping its results. */
  const close = (tabId: string) => {
    const runs = useQueryRunsStore.getState();
    if (runs.runningTab[sessionId] === tabId) void cancelRun(sessionId);
    runs.discard(tabId);
    actions.close(tabId);
  };

  /** Closes a tab, asking first if it still has SQL in it. */
  const requestClose = (tabId: string) => {
    const tab = tabSet?.tabs.find((t) => t.id === tabId);
    if (!tab) return;
    if (tab.sql.trim()) setConfirmingId(tab.id);
    else close(tab.id);
  };

  // While the dialog is open the shortcuts would act on tabs hidden behind it.
  useConsoleShortcuts({
    onNewTab: () => !confirming && actions.open(),
    onCloseTab: () => tabSet && !confirming && requestClose(tabSet.activeId),
    onCycleTab: (step) => !confirming && actions.cycle(step),
  });

  if (!tabSet) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <Spinner className="text-muted-foreground" />
      </div>
    );
  }

  return (
    <Tabs
      value={tabSet.activeId}
      onValueChange={(value) => actions.select(String(value))}
      className="min-h-0 flex-1 gap-0"
      style={{ "--tab-accent": colorVar(color) } as CSSProperties}
    >
      <ConsoleTabBar
        tabs={tabSet.tabs}
        activeId={tabSet.activeId}
        onNew={actions.open}
        onClose={requestClose}
        actions={
          <RunButton
            running={activeRun.running}
            busyElsewhere={activeRun.busyElsewhere}
            onRunStatement={activeRun.runStatement}
            onRunScript={activeRun.runScript}
            onCancel={activeRun.cancel}
          />
        }
      />
      {tabSet.tabs.map((tab) => (
        <TabsContent key={tab.id} value={tab.id} className="min-h-0">
          <ConsolePane
            tab={tab}
            sessionId={sessionId}
            driverId={driverId}
            onSqlChange={(sql) => actions.setSql(tab.id, sql)}
          />
        </TabsContent>
      ))}
      <CloseTabDialog
        tab={confirming}
        onConfirm={() => {
          if (confirming) close(confirming.id);
          setConfirmingId(null);
        }}
        onCancel={() => setConfirmingId(null)}
      />
    </Tabs>
  );
}
