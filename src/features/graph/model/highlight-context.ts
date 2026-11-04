// What the graph highlights: the hovered or selected table with its
// neighbours and the edges between them. Everything else dims.
import { createContext, useContext } from "react";

export type GraphHighlight = {
  /** Null = nothing highlighted, nothing dims. */
  nodes: ReadonlySet<string> | null;
  edges: ReadonlySet<string> | null;
  /** The edge under the pointer; it shows its label. */
  hoveredEdge: string | null;
};

export const NO_HIGHLIGHT: GraphHighlight = { nodes: null, edges: null, hoveredEdge: null };

export const GraphHighlightContext = createContext<GraphHighlight>(NO_HIGHLIGHT);

export function useGraphHighlightState(): GraphHighlight {
  return useContext(GraphHighlightContext);
}
