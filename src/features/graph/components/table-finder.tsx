import { SearchIcon } from "lucide-react";

import { TableKindIcon } from "@/components/schema/table-kind-icon";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { SHORTCUTS } from "@/features/console/model/shortcuts";
import type { TableEntry } from "@/features/tables/model/table-entry";

/** A searchable list of every table; picking one pans the graph to it. */
export function TableFinder({
  entries,
  open,
  onOpenChange,
  onFind,
}: {
  entries: TableEntry[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onFind: (entry: TableEntry) => void;
}) {
  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverTrigger render={<Button variant="outline" size="sm" className="w-56 justify-start text-muted-foreground" />}>
        <SearchIcon data-icon="inline-start" />
        Find table
        <kbd className="ml-auto font-sans text-[11px] text-muted-foreground/80">{SHORTCUTS.find}</kbd>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-72 p-0">
        <Command>
          <CommandInput placeholder="Table name" />
          <CommandList>
            <CommandEmpty>No table matches.</CommandEmpty>
            <CommandGroup>
              {entries.map((entry) => (
                <CommandItem
                  key={entry.key}
                  value={`${entry.schema}.${entry.table.name}`}
                  onSelect={() => {
                    onOpenChange(false);
                    onFind(entry);
                  }}
                >
                  <TableKindIcon kind={entry.table.kind} />
                  <span className="truncate">{entry.table.name}</span>
                  <span className="ml-auto truncate pl-2 font-mono text-[11px] text-muted-foreground">
                    {entry.schema}
                  </span>
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
