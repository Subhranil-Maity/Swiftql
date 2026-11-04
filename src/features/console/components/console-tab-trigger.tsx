import { XIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { TabsTrigger } from "@/components/ui/tabs";
import type { ConsoleTab } from "@/features/console/model/tab";

/**
 * One console tab. The close button is a sibling of the trigger, not a
 * child, because a button can't sit inside another button.
 */
export function ConsoleTabTrigger({
  tab,
  onClose,
}: {
  tab: ConsoleTab;
  onClose: () => void;
}) {
  return (
    <div className="group/tab relative flex h-full shrink-0 items-stretch">
      <TabsTrigger
        value={tab.id}
        title={tab.title}
        onAuxClick={(event) => {
          // Middle-click closes, as in browsers and editors.
          if (event.button === 1) onClose();
        }}
        className="h-full max-w-48 flex-none justify-start rounded-none px-3 pr-8 font-normal data-active:font-medium group-data-horizontal/tabs:after:bottom-0 group-data-[variant=line]/tabs-list:data-active:after:bg-(--tab-accent)"
      >
        <span className="truncate">{tab.title}</span>
      </TabsTrigger>
      <Button
        variant="ghost"
        size="icon-xs"
        aria-label={`Close ${tab.title}`}
        tabIndex={-1}
        onClick={onClose}
        className="absolute top-1/2 right-1.5 -translate-y-1/2 text-muted-foreground opacity-0 group-hover/tab:opacity-100 focus-visible:opacity-100 group-has-data-active/tab:opacity-100"
      >
        <XIcon />
      </Button>
    </div>
  );
}
