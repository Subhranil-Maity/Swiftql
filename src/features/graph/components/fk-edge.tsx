import { memo } from "react";
import {
  BaseEdge,
  type EdgeProps,
  EdgeLabelRenderer,
  getSmoothStepPath,
  type InternalNode,
  Position,
  useInternalNode,
} from "@xyflow/react";

import { EdgeEnds, type Outward } from "@/features/graph/components/edge-markers";
import type { FkEdge } from "@/features/graph/model/build-graph";
import { useGraphHighlightState } from "@/features/graph/model/highlight-context";
import { cn } from "@/lib/utils";

type Point = { x: number; y: number };

/** The same row's handle on the other side of the card: "col:id:r" ↔ "col:id:l". */
const otherSide = (handle: string) => (handle.endsWith(":r") ? `${handle.slice(0, -2)}:l` : `${handle.slice(0, -2)}:r`);

/** Where a handle sits on the canvas: the middle of the card's edge at that row. */
function handlePoint(node: InternalNode | undefined, handleId: string | null | undefined): Point | null {
  if (!node || !handleId) return null;
  const bounds = node.internals.handleBounds;
  const handle = [...(bounds?.source ?? []), ...(bounds?.target ?? [])].find((h) => h.id === handleId);
  if (!handle) return null;
  const origin = node.internals.positionAbsolute;
  return { x: origin.x + handle.x + handle.width / 2, y: origin.y + handle.y + handle.height / 2 };
}

/**
 * A foreign key: from the referenced row ("one", a bar) to the referencing
 * row ("many", a crow's foot). It leaves from whichever side of each card
 * faces the other, so dragged cards keep tidy edges.
 */
export const FkEdgeView = memo(function FkEdgeView({
  id,
  source,
  target,
  sourceHandleId,
  targetHandleId,
  sourceX,
  sourceY,
  targetX,
  targetY,
  data,
}: EdgeProps<FkEdge>) {
  const highlight = useGraphHighlightState();
  const sourceNode = useInternalNode(source);
  const targetNode = useInternalNode(target);

  const sRight = handlePoint(sourceNode, sourceHandleId) ?? { x: sourceX, y: sourceY };
  const sLeft = sourceHandleId ? (handlePoint(sourceNode, otherSide(sourceHandleId)) ?? sRight) : sRight;
  const tLeft = handlePoint(targetNode, targetHandleId) ?? { x: targetX, y: targetY };
  const tRight = targetHandleId ? (handlePoint(targetNode, otherSide(targetHandleId)) ?? tLeft) : tLeft;

  // Pick the facing sides; cards overlapping sideways (or a self-reference) loop round the right.
  let from: Point, to: Point, fromOut: Outward, toOut: Outward;
  if (source !== target && sRight.x <= tLeft.x) [from, fromOut, to, toOut] = [sRight, 1, tLeft, -1];
  else if (source !== target && tRight.x <= sLeft.x) [from, fromOut, to, toOut] = [sLeft, -1, tRight, 1];
  else [from, fromOut, to, toOut] = [sRight, 1, tRight, 1];

  const [path, labelX, labelY] = getSmoothStepPath({
    sourceX: from.x,
    sourceY: from.y,
    sourcePosition: fromOut === 1 ? Position.Right : Position.Left,
    targetX: to.x,
    targetY: to.y,
    targetPosition: toOut === 1 ? Position.Right : Position.Left,
    borderRadius: 6,
    offset: 18,
  });

  const active = highlight.edges?.has(id) ?? false;
  const dimmed = highlight.edges !== null && !active;
  const stroke = active ? "var(--tab-accent)" : "color-mix(in oklch, var(--border), var(--foreground) 22%)";
  const style = { stroke, strokeWidth: active ? 1.5 : 1.25, transition: "opacity 150ms" };
  const fk = data?.fk;

  return (
    <g className={cn(dimmed && "opacity-20")}>
      <BaseEdge
        id={id}
        path={path}
        interactionWidth={14}
        className={cn(active && "fk-edge-animated")}
        style={{
          ...style,
          ...(active && { strokeDasharray: "6 4", animation: "fk-dash 0.6s linear infinite" }),
        }}
      />
      <EdgeEnds
        one={{ ...from, out: fromOut }}
        many={{ ...to, out: toOut }}
        style={{ stroke, strokeWidth: style.strokeWidth }}
      />
      {fk && highlight.hoveredEdge === id && (
        <EdgeLabelRenderer>
          <div
            className="pointer-events-none absolute rounded-md border bg-popover px-2 py-1 font-mono text-[11px] text-popover-foreground shadow-sm"
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
          >
            {fk.name}
            <span className="text-muted-foreground">
              {" "}
              ({fk.columns.join(", ")}) → {fk.refTable}({fk.refColumns.join(", ")})
            </span>
          </div>
        </EdgeLabelRenderer>
      )}
    </g>
  );
});
