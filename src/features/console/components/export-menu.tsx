import { ChevronDownIcon, DownloadIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ExportFormat } from "@/features/console/model/result-export";

/** Saves every fetched row (in the current sort, visible columns only) to a file. */
export function ExportMenu({ onExport }: { onExport: (format: ExportFormat) => void }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button variant="ghost" size="xs" />}>
        <DownloadIcon data-icon="inline-start" />
        Export
        <ChevronDownIcon data-icon="inline-end" className="text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-40">
        <DropdownMenuGroup>
          <DropdownMenuItem onClick={() => onExport("csv")}>Export as CSV</DropdownMenuItem>
          <DropdownMenuItem onClick={() => onExport("json")}>Export as JSON</DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
