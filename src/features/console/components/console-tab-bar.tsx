import { type ReactNode, useEffect } from "react";
import { PlusIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { TabsList } from "@/components/ui/tabs";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { ConsoleTabTrigger } from "@/features/console/components/console-tab-trigger";
import { useHorizontalWheelScroll } from "@/features/console/hooks/use-horizontal-wheel-scroll";
import type { ConsoleTab } from "@/features/console/model/tab";
import { SHORTCUTS } from "@/features/console/model/shortcuts";

/** The strip of console tabs, the new-tab button and, on the right, `actions`. Must sit inside `Tabs`. */
export function ConsoleTabBar({
  tabs,
  activeId,
  onNew,
  onClose,
  actions,
}: {
  tabs: ConsoleTab[];
  activeId: string;
  onNew: () => void;
  onClose: (tabId: string) => void;
  /** Controls for the active tab, e.g. Run. */
  actions?: ReactNode;
}) {
  // The mouse wheel scrolls an overflowing strip sideways.
  const listRef = useHorizontalWheelScroll<HTMLDivElement>();

  // Keep the active tab visible, e.g. after Ctrl+T or Ctrl+Tab.
  useEffect(() => {
    listRef.current
      ?.querySelector('[role="tab"][aria-selected="true"]')
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [activeId, listRef]);

  return (
    <div className="flex h-9 shrink-0 items-stretch border-b">
      <TabsList
        ref={listRef}
        variant="line"
        aria-label="Console tabs"
        className="no-scrollbar h-full w-auto min-w-0 justify-start gap-0 overflow-x-auto p-0 group-data-horizontal/tabs:h-full"
      >
        {tabs.map((tab) => (
          <ConsoleTabTrigger key={tab.id} tab={tab} onClose={() => onClose(tab.id)} />
        ))}
      </TabsList>
      <div className="flex shrink-0 items-center px-1">
        <Tooltip>
          <TooltipTrigger
            render={<Button variant="ghost" size="icon-sm" aria-label="New tab" onClick={onNew} />}
          >
            <PlusIcon />
          </TooltipTrigger>
          <TooltipContent side="bottom">New tab ({SHORTCUTS.newTab})</TooltipContent>
        </Tooltip>
      </div>
      {actions && <div className="ml-auto flex shrink-0 items-center gap-1 px-2">{actions}</div>}
    </div>
  );
}
