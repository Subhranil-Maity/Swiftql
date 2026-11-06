import { useMemo } from "react";
import { toast } from "sonner";

import { run } from "@/features/console/hooks/use-run-query";
import { whenEditorReady } from "@/features/console/model/editor-registry";
import { selectAllSql } from "@/features/tables/model/generate-sql";
import { useWorkspace } from "@/features/workspace/model/workspace-context";
import { useConsoleTabsStore } from "@/stores/console-tabs";
import { useQueryRunsStore } from "@/stores/query-runs";
import { useWorkspaceViewStore } from "@/stores/workspace-view";

export type OpenSqlOptions = {
  title: string;
  sql: string;
  /** Run it as soon as the tab's editor exists. */
  runNow: boolean;
};

/**
 * Puts SQL in a new console tab and switches to the console. Shared by the
 * Tables list, its context menu and the graph's node menu.
 */
export function useOpenTableData() {
  const { connectionId, sessionId, driverId } = useWorkspace();

  return useMemo(() => {
    const openSql = async ({ title, sql, runNow }: OpenSqlOptions) => {
      const tabs = useConsoleTabsStore.getState();
      await tabs.load(connectionId);
      const tabId = useConsoleTabsStore.getState().openTab(connectionId, { title, sql });
      if (!tabId) {
        toast.error("Couldn't open a console tab");
        return;
      }
      useWorkspaceViewStore.getState().setView(connectionId, "console");
      if (!runNow) return;
      // One run per session: leave the query ready rather than queue it.
      if (useQueryRunsStore.getState().runningTab[sessionId]) {
        toast("Session busy. The query is in a new tab, ready to run.");
        return;
      }
      whenEditorReady(tabId, () => void run(tabId, sessionId, driverId, "script"));
    };

    return {
      openSql,
      /** Double-click / Enter / "Open data": the first rows in a new tab. */
      openTableData: (schema: string, table: string) =>
        openSql({ title: table, sql: selectAllSql(schema, table), runNow: true }),
    };
  }, [connectionId, sessionId, driverId]);
}
