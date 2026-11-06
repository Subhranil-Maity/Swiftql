import { type CSSProperties, useMemo } from "react";

import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { AppSidebar } from "@/features/workspace/components/app-sidebar";
import { WorkspaceHeader } from "@/features/workspace/components/workspace-header";
import { WorkspaceViews } from "@/features/workspace/components/workspace-views";
import { type Workspace, WorkspaceContext } from "@/features/workspace/model/workspace-context";
import type { SessionInfo } from "@/lib/ipc";
import { useConnectionsStore } from "@/stores/connections";

/** The workspace for one open session: sidebar, top bar and the current view. */
export function WorkspacePage({ session }: { session: SessionInfo }) {
  const connection = useConnectionsStore((s) =>
    s.connections.find((c) => c.id === session.connectionId),
  );
  const workspace = useMemo<Workspace>(
    () => ({
      connectionId: session.connectionId,
      connectionName: connection?.name ?? "Connection",
      sessionId: session.sessionId,
      driverId: session.driverId,
      color: connection?.color,
    }),
    [session.connectionId, session.sessionId, session.driverId, connection?.name, connection?.color],
  );

  return (
    <WorkspaceContext.Provider value={workspace}>
      <SidebarProvider
        className="h-svh min-h-0"
        // Narrower than shadcn's 16rem: the sidebar only holds short nav labels.
        style={{ "--sidebar-width": "12rem" } as CSSProperties}
      >
        <AppSidebar />
        <SidebarInset className="min-h-0 min-w-0 text-foreground">
          <WorkspaceHeader session={session} connection={connection} />
          {/* Keyed so switching connections starts from that connection's tabs and views. */}
          <WorkspaceViews key={session.connectionId} />
        </SidebarInset>
      </SidebarProvider>
    </WorkspaceContext.Provider>
  );
}
