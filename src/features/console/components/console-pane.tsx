import { useDefaultLayout } from "react-resizable-panels";

import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { OutputPanel } from "@/features/console/components/output-panel";
import { SqlEditor } from "@/features/console/components/sql-editor";
import { useRunQuery } from "@/features/console/hooks/use-run-query";
import type { ConsoleTab } from "@/features/console/model/tab";

/** One tab's body: the editor on top, the output below, split by a draggable handle. */
export function ConsolePane({
  tab,
  sessionId,
  driverId,
  onSqlChange,
}: {
  tab: ConsoleTab;
  sessionId: string;
  driverId: string;
  onSqlChange: (sql: string) => void;
}) {
  // Shared by every tab and remembered in localStorage: it's a UI preference, not tab data.
  const layout = useDefaultLayout({ id: "console-split" });
  const { runStatement, runScript } = useRunQuery(tab.id, sessionId, driverId);

  return (
    <ResizablePanelGroup
      orientation="vertical"
      defaultLayout={layout.defaultLayout}
      onLayoutChanged={layout.onLayoutChanged}
    >
      <ResizablePanel id="editor" defaultSize="50" minSize="20">
        <SqlEditor
          tabId={tab.id}
          sessionId={sessionId}
          value={tab.sql}
          onChange={onSqlChange}
          label={tab.title}
          driverId={driverId}
          onRunStatement={runStatement}
          onRunScript={runScript}
        />
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel id="output" defaultSize="50" minSize="15">
        <OutputPanel tabId={tab.id} />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
