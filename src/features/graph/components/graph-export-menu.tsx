import { ChevronDownIcon, DownloadIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { GraphExportFormat } from "@/features/graph/hooks/use-graph-export";

export function GraphExportMenu({
  onExport,
  disabled,
}: {
  onExport: (format: GraphExportFormat) => void;
  disabled?: boolean;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger disabled={disabled} render={<Button variant="outline" size="sm" />}>
        <DownloadIcon data-icon="inline-start" />
        Export
        <ChevronDownIcon data-icon="inline-end" className="text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-40">
        <DropdownMenuGroup>
          <DropdownMenuItem onClick={() => onExport("png")}>Export as PNG</DropdownMenuItem>
          <DropdownMenuItem onClick={() => onExport("svg")}>Export as SVG</DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
