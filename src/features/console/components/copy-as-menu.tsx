import { ChevronDownIcon, CopyIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { type ExportFormat, FORMAT_LABELS } from "@/features/console/model/result-export";

const FORMATS: ExportFormat[] = ["csv", "tsv", "markdown", "json"];

/** Copies the selection, or every row if nothing is selected, in the picked format. */
export function CopyAsMenu({
  hasSelection,
  onCopy,
}: {
  hasSelection: boolean;
  onCopy: (format: ExportFormat) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button variant="ghost" size="xs" />}>
        <CopyIcon data-icon="inline-start" />
        Copy as
        <ChevronDownIcon data-icon="inline-end" className="text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-44">
        <DropdownMenuGroup>
          <DropdownMenuLabel>{hasSelection ? "Selected cells" : "All rows"}</DropdownMenuLabel>
          {FORMATS.map((format) => (
            <DropdownMenuItem key={format} onClick={() => onCopy(format)}>
              {FORMAT_LABELS[format]}
            </DropdownMenuItem>
          ))}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
