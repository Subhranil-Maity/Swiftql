import { useMemo } from "react";
import { toast } from "sonner";

import { useOpenTableData } from "@/features/tables/hooks/use-open-table-data";
import { insertSql, selectColumnsSql } from "@/features/tables/model/generate-sql";
import type { TableEntry } from "@/features/tables/model/table-entry";
import { useWorkspace } from "@/features/workspace/model/workspace-context";
import { displayQualifiedName } from "@/lib/sql-ident";
import { useWorkspaceViewStore } from "@/stores/workspace-view";

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast.success(`Copied ${text}`);
  } catch {
    toast.error("Couldn't copy to the clipboard");
  }
}

/** What a table's context menu can do; shared by the Tables list and the graph. */
export function useTableActions() {
  const { connectionId } = useWorkspace();
  const { openSql, openTableData } = useOpenTableData();

  return useMemo(
    () => ({
      openData: (e: TableEntry) => void openTableData(e.schema, e.table.name),
      copyName: (e: TableEntry) => void copy(e.table.name),
      copyQualifiedName: (e: TableEntry) => void copy(displayQualifiedName(e.schema, e.table.name)),
      newSelect: (e: TableEntry) =>
        void openSql({
          title: e.table.name,
          sql: selectColumnsSql(e.schema, e.table.name, e.table.columns ?? []),
          runNow: false,
        }),
      newInsert: (e: TableEntry) =>
        void openSql({
          title: `Insert into ${e.table.name}`,
          sql: insertSql(e.schema, e.table.name, e.table.columns ?? []),
          runNow: false,
        }),
      showInGraph: (e: TableEntry) => useWorkspaceViewStore.getState().showInGraph(connectionId, e.key),
    }),
    [connectionId, openSql, openTableData],
  );
}

export type TableActions = ReturnType<typeof useTableActions>;
