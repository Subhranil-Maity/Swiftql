import type { LucideIcon } from "lucide-react";

import { SidebarMenuBadge, SidebarMenuButton, SidebarMenuItem } from "@/components/ui/sidebar";

/** One workspace view in the sidebar; the label doubles as the icon rail's tooltip. */
export function SidebarNavItem({
  icon: Icon,
  label,
  active,
  onSelect,
  badge,
}: {
  icon: LucideIcon;
  label: string;
  active: boolean;
  onSelect: () => void;
  /** e.g. unread notifications; hidden when 0. */
  badge?: number;
}) {
  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        isActive={active}
        tooltip={label}
        aria-current={active ? "page" : undefined}
        onClick={onSelect}
      >
        <Icon />
        <span>{label}</span>
      </SidebarMenuButton>
      {badge ? <SidebarMenuBadge aria-label={`${badge} unread`}>{badge > 99 ? "99+" : badge}</SidebarMenuBadge> : null}
    </SidebarMenuItem>
  );
}
