import type { ReactNode } from "react";
import { ChevronRightIcon } from "lucide-react";

import { Collapsible, CollapsibleContent } from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";

/** A schema's header row (name, table count) and, when open, its tables. */
export function SchemaGroup({
  schema,
  count,
  open,
  onOpenChange,
  treeProps,
  children,
}: {
  schema: string;
  /** Tables shown; while searching, the matches. */
  count: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  treeProps: { "data-tree-id": string; tabIndex: number; onFocus: () => void };
  children: ReactNode;
}) {
  return (
    <Collapsible open={open} onOpenChange={onOpenChange} render={<div role="none" />}>
      <div
        role="treeitem"
        aria-level={1}
        aria-expanded={open}
        {...treeProps}
        onClick={() => onOpenChange(!open)}
        className="flex h-7 cursor-default items-center gap-1.5 rounded-md px-2 font-medium outline-none select-none hover:bg-muted/60 focus-visible:bg-muted focus-visible:shadow-[inset_2px_0_0_var(--tab-accent)]"
      >
        <ChevronRightIcon
          aria-hidden
          className={cn(
            "size-3.5 shrink-0 text-muted-foreground transition-transform duration-150 motion-reduce:transition-none",
            open && "rotate-90",
          )}
        />
        <span className="min-w-0 truncate">{schema}</span>
        <span className="ml-auto pl-4 text-xs font-normal text-muted-foreground tabular-nums">
          {count}
        </span>
      </div>
      <CollapsibleContent render={<div role="group" aria-label={schema} />}>{children}</CollapsibleContent>
    </Collapsible>
  );
}
