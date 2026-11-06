import type { Ref } from "react";
import { CircleAlertIcon, RefreshCwIcon } from "lucide-react";

import { SearchInput } from "@/components/schema/search-input";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { plural } from "@/lib/format-count";
import { cn } from "@/lib/utils";

/** Filter box, counts and Refresh. A failed refresh shows as a warning icon. */
export function TablesToolbar({
  query,
  onQueryChange,
  searchRef,
  counts,
  searching,
  loading,
  error,
  onRefresh,
}: {
  query: string;
  onQueryChange: (query: string) => void;
  searchRef?: Ref<HTMLInputElement>;
  counts: { schemas: number; tables: number; matches: number };
  searching: boolean;
  loading: boolean;
  /** The last refresh's error, while older data still shows. */
  error: string | null;
  onRefresh: () => void;
}) {
  return (
    <div className="flex h-11 shrink-0 items-center gap-3 border-b px-4">
      <SearchInput
        ref={searchRef}
        value={query}
        onChange={onQueryChange}
        placeholder="Filter tables"
      />
      <span className="ml-auto text-xs text-muted-foreground tabular-nums" aria-live="polite">
        {searching
          ? `${counts.matches.toLocaleString()} of ${plural(counts.tables, "table")}`
          : `${plural(counts.schemas, "schema")} · ${plural(counts.tables, "table")}`}
      </span>
      {error && (
        <Tooltip>
          <TooltipTrigger render={<span className="text-warning" />}>
            <CircleAlertIcon aria-label="Refresh failed" className="size-4" />
          </TooltipTrigger>
          <TooltipContent side="bottom">Couldn't refresh: {error}</TooltipContent>
        </Tooltip>
      )}
      <Tooltip>
        <TooltipTrigger
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Refresh"
              onClick={onRefresh}
              disabled={loading}
            />
          }
        >
          <RefreshCwIcon className={cn(loading && "animate-spin motion-reduce:animate-none")} />
        </TooltipTrigger>
        <TooltipContent side="bottom">Refresh</TooltipContent>
      </Tooltip>
    </div>
  );
}
