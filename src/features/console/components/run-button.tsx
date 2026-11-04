import { ChevronDownIcon, PlayIcon, SquareIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ButtonGroup, ButtonGroupSeparator } from "@/components/ui/button-group";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { SHORTCUTS } from "@/features/console/model/shortcuts";

/**
 * Runs the active tab's SQL. The main part runs the statement at the cursor
 * (or the selection) and turns into Stop while it runs; the menu also has
 * Run script.
 */
export function RunButton({
  running,
  busyElsewhere,
  onRunStatement,
  onRunScript,
  onCancel,
}: {
  running: boolean;
  /** Another tab of this session is running, so this one has to wait. */
  busyElsewhere: boolean;
  onRunStatement: () => void;
  onRunScript: () => void;
  onCancel: () => void;
}) {
  if (running) {
    return (
      <Button size="sm" variant="secondary" onClick={onCancel}>
        <SquareIcon data-icon="inline-start" className="fill-current" />
        Stop
      </Button>
    );
  }

  const group = (
    <ButtonGroup aria-label="Run">
      <Tooltip disabled={busyElsewhere}>
        <TooltipTrigger
          render={
            <Button
              size="sm"
              disabled={busyElsewhere}
              tabIndex={busyElsewhere ? -1 : undefined}
              onClick={onRunStatement}
            />
          }
        >
          <PlayIcon data-icon="inline-start" />
          Run
        </TooltipTrigger>
        <TooltipContent side="bottom">Run statement ({SHORTCUTS.runStatement})</TooltipContent>
      </Tooltip>
      <ButtonGroupSeparator className="bg-primary-foreground/25" />
      <DropdownMenu>
        <DropdownMenuTrigger
          disabled={busyElsewhere}
          render={<Button size="icon-sm" aria-label="Run options" tabIndex={busyElsewhere ? -1 : undefined} />}
        >
          <ChevronDownIcon />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-60">
          <DropdownMenuItem onClick={onRunStatement}>
            Run statement
            <DropdownMenuShortcut>{SHORTCUTS.runStatement}</DropdownMenuShortcut>
          </DropdownMenuItem>
          <DropdownMenuItem onClick={onRunScript}>
            Run script
            <DropdownMenuShortcut>{SHORTCUTS.runScript}</DropdownMenuShortcut>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </ButtonGroup>
  );

  if (!busyElsewhere) return group;

  return (
    <Tooltip>
      {/* Disabled buttons don't fire pointer events, so the span carries the tooltip. */}
      <TooltipTrigger
        render={<span tabIndex={0} />}
        className="rounded-md outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
      >
        {group}
      </TooltipTrigger>
      <TooltipContent side="bottom">Another tab of this connection is running a query</TooltipContent>
    </Tooltip>
  );
}
