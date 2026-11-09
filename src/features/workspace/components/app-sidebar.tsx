import {
  BellIcon,
  type LucideIcon,
  NetworkIcon,
  SettingsIcon,
  SquareTerminalIcon,
  TableIcon,
} from "lucide-react";

import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { useNotificationsStore } from "@/features/notifications/stores/notifications";
import { SidebarNavItem } from "@/features/workspace/components/sidebar-nav-item";
import { useViewAvailable } from "@/features/workspace/model/available-views";
import { useWorkspace } from "@/features/workspace/model/workspace-context";
import { useNavigationStore } from "@/stores/navigation";
import { type WorkspaceView, useWorkspaceView, useWorkspaceViewStore } from "@/stores/workspace-view";

const VIEWS: { view: WorkspaceView; label: string; icon: LucideIcon }[] = [
  { view: "console", label: "Console", icon: SquareTerminalIcon },
  { view: "tables", label: "Tables", icon: TableIcon },
  { view: "graph", label: "Graph", icon: NetworkIcon },
  { view: "notifications", label: "Notifications", icon: BellIcon },
];

/** Workspace navigation. Collapses to an icon rail (the header toggle or Ctrl+B). */
export function AppSidebar() {
  const go = useNavigationStore((s) => s.go);
  const { connectionId, sessionId, driverId } = useWorkspace();
  const unread = useNotificationsStore((s) => s.bySession[sessionId]?.unread ?? 0);
  const current = useWorkspaceView(connectionId);
  const setView = useWorkspaceViewStore((s) => s.setView);
  const available = useViewAvailable(driverId);

  return (
    <Sidebar collapsible="icon">
      {/* Same height as the workspace top bar so the two line up. */}
      <SidebarHeader className="h-12 flex-row items-center justify-between">
        <span className="truncate pl-2 font-semibold tracking-tight group-data-[collapsible=icon]:hidden">
          SwiftQL
        </span>
        <SidebarTrigger size="icon" className="text-muted-foreground" />
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup className="pt-0">
          <SidebarMenu aria-label="Views">
            {/* TODO: a query history view will join these */}
            {VIEWS.filter(({ view }) => available(view)).map(({ view, label, icon }) => (
              <SidebarNavItem
                key={view}
                icon={icon}
                label={label}
                active={current === view}
                onSelect={() => setView(connectionId, view)}
                badge={view === "notifications" ? unread : undefined}
              />
            ))}
          </SidebarMenu>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton tooltip="Settings" onClick={() => go({ page: "settings" })}>
              <SettingsIcon />
              <span>Settings</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
