import { useMemo, useState } from "react";
import type { Edge, Node, OnSelectionChangeParams } from "@xyflow/react";

import { type GraphHighlight, NO_HIGHLIGHT } from "@/features/graph/model/highlight-context";

/**
 * Hovering (or selecting) a table highlights it, the tables it's linked to
 * and the edges between them. Hovering an edge highlights it and its ends.
 */
export function useGraphHighlight(edges: Edge[]) {
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);
  const [hoveredEdge, setHoveredEdge] = useState<string | null>(null);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);

  const highlight = useMemo<GraphHighlight>(() => {
    const focus = hoveredNode ?? selectedNode;
    if (focus) {
      const linked = edges.filter((e) => e.source === focus || e.target === focus);
      return {
        nodes: new Set([focus, ...linked.flatMap((e) => [e.source, e.target])]),
        edges: new Set(linked.map((e) => e.id)),
        hoveredEdge,
      };
    }
    const edge = hoveredEdge ? edges.find((e) => e.id === hoveredEdge) : undefined;
    if (edge) return { nodes: new Set([edge.source, edge.target]), edges: new Set([edge.id]), hoveredEdge };
    return NO_HIGHLIGHT;
  }, [edges, hoveredNode, hoveredEdge, selectedNode]);

  const handlers = useMemo(
    () => ({
      onNodeMouseEnter: (_: unknown, node: Node) => setHoveredNode(node.id),
      onNodeMouseLeave: () => setHoveredNode(null),
      onEdgeMouseEnter: (_: unknown, edge: Edge) => setHoveredEdge(edge.id),
      onEdgeMouseLeave: () => setHoveredEdge(null),
      onSelectionChange: ({ nodes }: OnSelectionChangeParams) =>
        setSelectedNode(nodes.length === 1 ? nodes[0].id : null),
    }),
    [],
  );

  return { highlight, handlers };
}
