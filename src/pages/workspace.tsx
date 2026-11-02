import type { CSSProperties } from "react";

import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { ConsoleView } from "@/features/console/components/console-view";
import { AppSidebar } from "@/features/workspace/components/app-sidebar";
import { WorkspaceHeader } from "@/features/workspace/components/workspace-header";
import type { SessionInfo } from "@/lib/ipc";
import { useConnectionsStore } from "@/stores/connections";

/** The workspace for one open session: sidebar, top bar and the console. */
export function WorkspacePage({ session }: { session: SessionInfo }) {
  const connection = useConnectionsStore((s) =>
    s.connections.find((c) => c.id === session.connectionId),
  );

  return (
    <SidebarProvider
      className="h-svh min-h-0"
      // Narrower than shadcn's 16rem: the sidebar only holds short nav labels.
      style={{ "--sidebar-width": "12rem" } as CSSProperties}
    >
      <AppSidebar />
      <SidebarInset className="min-h-0 min-w-0 text-foreground">
        <WorkspaceHeader session={session} connection={connection} />
        {/* Keyed so switching connections starts from that connection's tabs. */}
        <ConsoleView
          key={session.connectionId}
          connectionId={session.connectionId}
          driverId={session.driverId}
          color={connection?.color}
        />
      </SidebarInset>
    </SidebarProvider>
  );
}
