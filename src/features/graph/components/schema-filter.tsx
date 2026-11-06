import { CheckIcon, ChevronDownIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { cn } from "@/lib/utils";

/** Which schemas the graph shows. At least one stays picked. */
export function SchemaFilter({
  schemas,
  selected,
  onChange,
  searchPath,
}: {
  schemas: string[];
  selected: ReadonlySet<string>;
  onChange: (schemas: string[]) => void;
  searchPath: string[];
}) {
  const label =
    selected.size === 1 ? [...selected][0] : selected.size === schemas.length ? "All schemas" : `${selected.size} schemas`;

  const toggle = (schema: string) => {
    const next = new Set(selected);
    if (next.has(schema)) {
      if (next.size === 1) return;
      next.delete(schema);
    } else {
      next.add(schema);
    }
    // Keep the model's order so the saved list is stable.
    onChange(schemas.filter((s) => next.has(s)));
  };
  const onPath = schemas.filter((s) => searchPath.includes(s));

  return (
    <Popover>
      <PopoverTrigger render={<Button variant="outline" size="sm" />}>
        <span className="max-w-40 truncate">{label}</span>
        <ChevronDownIcon data-icon="inline-end" className="text-muted-foreground" />
      </PopoverTrigger>
      <PopoverContent align="start" className="w-60 p-0">
        <Command>
          {schemas.length > 8 && <CommandInput placeholder="Find schema" />}
          <CommandList>
            <CommandEmpty>No schema matches.</CommandEmpty>
            <CommandGroup heading="Schemas">
              {schemas.map((schema) => (
                <CommandItem key={schema} value={schema} onSelect={() => toggle(schema)}>
                  <CheckIcon className={cn("size-3.5", !selected.has(schema) && "opacity-0")} />
                  <span className="truncate">{schema}</span>
                </CommandItem>
              ))}
            </CommandGroup>
            <CommandSeparator />
            <CommandGroup>
              <CommandItem value="__all" onSelect={() => onChange(schemas)}>
                Show all schemas
              </CommandItem>
              {onPath.length > 0 && (
                <CommandItem value="__path" onSelect={() => onChange(onPath)}>
                  Only the search path ({onPath.join(", ")})
                </CommandItem>
              )}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
