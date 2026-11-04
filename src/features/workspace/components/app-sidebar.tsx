import { SettingsIcon, SquareTerminalIcon } from "lucide-react";

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
import { useNavigationStore } from "@/stores/navigation";

/** Workspace navigation. Collapses to an icon rail (the header toggle or Ctrl+B). */
export function AppSidebar() {
  const go = useNavigationStore((s) => s.go);

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
          <SidebarMenu>
            {/* TODO: more views (schema explorer, history) will join Console here */}
            <SidebarMenuItem>
              <SidebarMenuButton isActive tooltip="Console">
                <SquareTerminalIcon />
                <span>Console</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
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
