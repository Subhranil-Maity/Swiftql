import { useEffect, useMemo, useState } from "react";

import type { GraphData, GraphNode } from "@/features/graph/model/build-graph";
import { layoutGraph, type XY } from "@/features/graph/model/layout";
import { errorMessage } from "@/lib/ipc-error";

/** Changes when anything elk looks at changes: nodes, their sizes, edges. */
function layoutSignature(graph: GraphData): string {
  const nodes = graph.nodes.map((n) => `${n.id}:${n.data.size.width}x${n.data.size.height}`).join("|");
  return `${nodes}#${graph.edges.map((e) => `${e.source}>${e.target}`).join("|")}`;
}

/**
 * Positions for the graph's nodes: the user's saved drags win, elk places
 * the rest. Nodes without either yet stay hidden until elk answers.
 */
export function useAutoLayout(graph: GraphData, saved: Record<string, XY>, enabled: boolean) {
  const signature = useMemo(() => layoutSignature(graph), [graph]);
  const [auto, setAuto] = useState<{ signature: string; positions: Record<string, XY> } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const allSaved = graph.nodes.every((n) => saved[n.id]);

  useEffect(() => {
    if (!enabled || allSaved || auto?.signature === signature) return;
    let cancelled = false;
    layoutGraph(graph.nodes, graph.edges)
      .then((positions) => {
        if (cancelled) return;
        setAuto({ signature, positions });
        setError(null);
      })
      .catch((e: unknown) => !cancelled && setError(errorMessage(e)));
    return () => {
      cancelled = true;
    };
    // `signature` stands in for `graph`: a new object with the same layout inputs needs no new run.
  }, [signature, enabled, allSaved]);

  const nodes = useMemo<GraphNode[]>(
    () =>
      graph.nodes.map((n) => {
        const position = saved[n.id] ?? auto?.positions[n.id];
        return { ...n, position: position ?? { x: 0, y: 0 }, hidden: !position };
      }),
    [graph.nodes, saved, auto],
  );

  const ready = enabled && (allSaved || auto?.signature === signature);
  return { nodes, ready, error };
}
