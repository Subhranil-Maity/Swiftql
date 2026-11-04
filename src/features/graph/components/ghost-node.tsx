import { memo } from "react";
import type { NodeProps } from "@xyflow/react";

import { RowHandles } from "@/features/graph/components/column-handle";
import { cardHandle, type GhostNode } from "@/features/graph/model/build-graph";
import { useGraphHighlightState } from "@/features/graph/model/highlight-context";
import { cn } from "@/lib/utils";

/** A referenced table from a schema that's filtered out: a dashed stub with its name. */
export const GhostNodeView = memo(function GhostNodeView({ id, data }: NodeProps<GhostNode>) {
  const highlight = useGraphHighlightState();
  const dimmed = highlight.nodes !== null && !highlight.nodes.has(id);
  return (
    <div
      style={{ width: data.size.width, height: data.size.height }}
      title={`${data.schema} isn't shown. Add it in Schemas to see this table.`}
      className={cn(
        "relative flex items-center rounded-md border border-dashed bg-background/60 px-2.5 text-xs text-muted-foreground transition-opacity duration-150 motion-reduce:transition-none",
        dimmed && "opacity-35",
      )}
    >
      <span className="truncate font-mono text-[11px]">
        {data.schema}.{data.table}
      </span>
      <RowHandles left={cardHandle("l")} right={cardHandle("r")} />
    </div>
  );
});
