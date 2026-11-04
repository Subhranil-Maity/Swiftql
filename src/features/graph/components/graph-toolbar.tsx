import { LayoutGridIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { GraphExportMenu } from "@/features/graph/components/graph-export-menu";
import { SchemaFilter } from "@/features/graph/components/schema-filter";
import { TableFinder } from "@/features/graph/components/table-finder";
import type { GraphExportFormat } from "@/features/graph/hooks/use-graph-export";
import type { TableEntry } from "@/features/tables/model/table-entry";

/** Schemas and Find on the left; Re-layout and Export on the right. */
export function GraphToolbar({
  schemas,
  selected,
  searchPath,
  onSchemasChange,
  entries,
  finderOpen,
  onFinderOpenChange,
  onFind,
  onRelayout,
  onExport,
  empty,
}: {
  schemas: string[];
  selected: ReadonlySet<string>;
  searchPath: string[];
  onSchemasChange: (schemas: string[]) => void;
  entries: TableEntry[];
  finderOpen: boolean;
  onFinderOpenChange: (open: boolean) => void;
  onFind: (entry: TableEntry) => void;
  onRelayout: () => void;
  onExport: (format: GraphExportFormat) => void;
  /** Nothing on the canvas to lay out or export. */
  empty: boolean;
}) {
  return (
    <div className="flex h-11 shrink-0 items-center gap-2 border-b px-4">
      <SchemaFilter schemas={schemas} selected={selected} onChange={onSchemasChange} searchPath={searchPath} />
      <TableFinder entries={entries} open={finderOpen} onOpenChange={onFinderOpenChange} onFind={onFind} />
      <div className="ml-auto flex items-center gap-2">
        <Button variant="ghost" size="sm" onClick={onRelayout} disabled={empty}>
          <LayoutGridIcon data-icon="inline-start" />
          Re-layout
        </Button>
        <GraphExportMenu onExport={onExport} disabled={empty} />
      </div>
    </div>
  );
}
