import { type ReactElement, useRef, useState } from "react";
import {
  CopyIcon,
  FileInputIcon,
  InfoIcon,
  NetworkIcon,
  PlayIcon,
  TextCursorInputIcon,
} from "lucide-react";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { useTableActions } from "@/features/tables/hooks/use-table-actions";
import type { TableEntry } from "@/features/tables/model/table-entry";

/**
 * One right-click menu for every table inside `children` (the list, or the
 * graph canvas). `resolve` finds the table under the pointer; right-clicking
 * anywhere else opens nothing.
 */
export function TableContextMenu({
  children,
  resolve,
  onViewDetails,
  showInGraph = true,
}: {
  /** The container; rendered as the menu's trigger. */
  children: ReactElement;
  resolve: (target: Element) => TableEntry | undefined;
  onViewDetails: (entry: TableEntry) => void;
  /** Off inside the graph itself. */
  showInGraph?: boolean;
}) {
  const actions = useTableActions();
  const [entry, setEntry] = useState<TableEntry | null>(null);
  const [open, setOpen] = useState(false);
  // Read by onOpenChange in the same event, before the state update lands.
  const pending = useRef<TableEntry | undefined>(undefined);

  const run = (action: (e: TableEntry) => void) => () => {
    if (entry) action(entry);
  };

  return (
    <ContextMenu
      open={open}
      onOpenChange={(next) => {
        if (next && !pending.current) return;
        setOpen(next);
      }}
    >
      <ContextMenuTrigger
        render={children}
        onContextMenuCapture={(event) => {
          pending.current =
            event.target instanceof Element ? resolve(event.target) : undefined;
          if (pending.current) setEntry(pending.current);
        }}
      />
      <ContextMenuContent className="w-52">
        <ContextMenuGroup>
          <ContextMenuItem onClick={run(actions.openData)}>
            <PlayIcon />
            Open data
          </ContextMenuItem>
          <ContextMenuItem onClick={run(onViewDetails)}>
            <InfoIcon />
            View details
          </ContextMenuItem>
          {showInGraph && (
            <ContextMenuItem onClick={run(actions.showInGraph)}>
              <NetworkIcon />
              Show in graph
            </ContextMenuItem>
          )}
        </ContextMenuGroup>
        <ContextMenuSeparator />
        <ContextMenuGroup>
          <ContextMenuItem onClick={run(actions.newSelect)}>
            <TextCursorInputIcon />
            New SELECT
          </ContextMenuItem>
          <ContextMenuItem onClick={run(actions.newInsert)}>
            <FileInputIcon />
            New INSERT
          </ContextMenuItem>
        </ContextMenuGroup>
        <ContextMenuSeparator />
        <ContextMenuGroup>
          <ContextMenuItem onClick={run(actions.copyName)}>
            <CopyIcon />
            Copy name
          </ContextMenuItem>
          <ContextMenuItem onClick={run(actions.copyQualifiedName)}>
            <CopyIcon />
            Copy qualified name
          </ContextMenuItem>
        </ContextMenuGroup>
      </ContextMenuContent>
    </ContextMenu>
  );
}
