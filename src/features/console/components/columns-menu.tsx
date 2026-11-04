import { Columns3Icon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ColumnMeta } from "@/lib/ipc";

/** Show or hide result columns; the last visible one can't be hidden. */
export function ColumnsMenu({
  columns,
  hidden,
  onToggle,
  onShowAll,
}: {
  columns: ColumnMeta[];
  hidden: ReadonlySet<number>;
  onToggle: (column: number) => void;
  onShowAll: () => void;
}) {
  const lastVisible = columns.length - hidden.size === 1;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button variant="ghost" size="xs" />}>
        <Columns3Icon data-icon="inline-start" />
        Columns
        {hidden.size > 0 && (
          <span className="text-muted-foreground tabular-nums">
            {columns.length - hidden.size}/{columns.length}
          </span>
        )}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="max-h-80 w-56">
        <DropdownMenuGroup>
          {columns.map((column, i) => (
            <DropdownMenuCheckboxItem
              key={i}
              checked={!hidden.has(i)}
              disabled={lastVisible && !hidden.has(i)}
              closeOnClick={false}
              onCheckedChange={() => onToggle(i)}
            >
              <span className="min-w-0 truncate">{column.name}</span>
              <span className="ml-auto truncate pl-3 font-mono text-[11px] text-muted-foreground">
                {column.nativeType}
              </span>
            </DropdownMenuCheckboxItem>
          ))}
        </DropdownMenuGroup>
        {hidden.size > 0 && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={onShowAll}>Show all columns</DropdownMenuItem>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
