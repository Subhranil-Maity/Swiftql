import { PlayIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

/** Runs the active tab's SQL. Disabled until the backend can execute queries. */
export function RunButton() {
  return (
    <Tooltip>
      {/* Disabled buttons don't fire pointer events, so the span carries the tooltip. */}
      <TooltipTrigger
        render={<span tabIndex={0} />}
        className="rounded-md outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
      >
        {/* TODO: run the active tab's SQL through api once the execute command exists; bind SHORTCUTS.run */}
        <Button size="sm" disabled tabIndex={-1}>
          <PlayIcon data-icon="inline-start" />
          Run
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">Running queries isn't available yet</TooltipContent>
    </Tooltip>
  );
}
