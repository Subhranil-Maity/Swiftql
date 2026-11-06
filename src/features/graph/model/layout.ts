// Auto-layout with elk's layered algorithm, run in a web worker so large
// schemas don't freeze the UI. Referenced tables end up left of the tables
// that point at them.
import ELK, { type ElkNode } from "elkjs/lib/elk-api";
import elkWorkerUrl from "elkjs/lib/elk-worker.min.js?url";

import type { FkEdge, GraphNode } from "@/features/graph/model/build-graph";

export type XY = { x: number; y: number };

let elk: InstanceType<typeof ELK> | null = null;

const OPTIONS: Record<string, string> = {
  "elk.algorithm": "layered",
  "elk.direction": "RIGHT",
  "elk.edgeRouting": "ORTHOGONAL",
  "elk.layered.spacing.nodeNodeBetweenLayers": "90",
  "elk.spacing.nodeNode": "36",
  "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
  "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
  // Unrelated tables are packed into a block rather than one long row.
  "elk.separateConnectedComponents": "true",
  "elk.spacing.componentComponent": "64",
  "elk.aspectRatio": "1.6",
};

/** Top-left positions for every node, by id. */
export async function layoutGraph(nodes: GraphNode[], edges: FkEdge[]): Promise<Record<string, XY>> {
  if (nodes.length === 0) return {};
  elk ??= new ELK({ workerUrl: elkWorkerUrl });
  const ids = new Set(nodes.map((n) => n.id));
  const graph: ElkNode = {
    id: "root",
    layoutOptions: OPTIONS,
    children: nodes.map((n) => ({ id: n.id, width: n.data.size.width, height: n.data.size.height })),
    edges: edges
      // elk rejects edges to unknown nodes; self-references add nothing to the layout.
      .filter((e) => ids.has(e.source) && ids.has(e.target) && e.source !== e.target)
      .map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] })),
  };
  const result = await elk.layout(graph);
  const positions: Record<string, XY> = {};
  for (const child of result.children ?? []) {
    positions[child.id] = { x: Math.round(child.x ?? 0), y: Math.round(child.y ?? 0) };
  }
  return positions;
}
