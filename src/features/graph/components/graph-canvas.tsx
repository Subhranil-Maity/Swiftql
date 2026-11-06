import "@xyflow/react/dist/style.css";

import { useEffect } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  type EdgeTypes,
  MiniMap,
  type NodeTypes,
  ReactFlow,
  useNodesState,
} from "@xyflow/react";

import { FkEdgeView } from "@/features/graph/components/fk-edge";
import { GhostNodeView } from "@/features/graph/components/ghost-node";
import { TableNodeView } from "@/features/graph/components/table-node";
import { useGraphHighlight } from "@/features/graph/hooks/use-graph-highlight";
import type { FkEdge, GraphNode } from "@/features/graph/model/build-graph";
import type { XY } from "@/features/graph/model/layout";
import { GraphHighlightContext } from "@/features/graph/model/highlight-context";

const NODE_TYPES: NodeTypes = { table: TableNodeView, ghost: GhostNodeView };
const EDGE_TYPES: EdgeTypes = { fk: FkEdgeView };

/** The React Flow canvas: dot grid, zoom controls, minimap, draggable table cards. */
export function GraphCanvas({
  nodes: laidOut,
  edges,
  onDragged,
  onOpenTable,
}: {
  /** Positioned nodes; a change here (layout, refresh) replaces the canvas's copy. */
  nodes: GraphNode[];
  edges: FkEdge[];
  /** Cards the user moved, with their new positions. */
  onDragged: (positions: Record<string, XY>) => void;
  onOpenTable: (key: string) => void;
}) {
  const [nodes, setNodes, onNodesChange] = useNodesState<GraphNode>(laidOut);
  const { highlight, handlers } = useGraphHighlight(edges);

  // Take new positions and data, keep what React Flow tracks itself (selection, measured size).
  useEffect(() => {
    setNodes((current) => {
      const previous = new Map(current.map((n) => [n.id, n]));
      return laidOut.map((n) => {
        const old = previous.get(n.id);
        return old ? ({ ...n, selected: old.selected, measured: old.measured } as GraphNode) : n;
      });
    });
  }, [laidOut, setNodes]);

  return (
    <GraphHighlightContext.Provider value={highlight}>
      <ReactFlow<GraphNode, FkEdge>
        className="swiftql-flow"
        nodes={nodes}
        edges={edges}
        nodeTypes={NODE_TYPES}
        edgeTypes={EDGE_TYPES}
        onNodesChange={onNodesChange}
        onNodeDragStop={(_, __, dragged) =>
          onDragged(Object.fromEntries(dragged.map((n) => [n.id, { x: Math.round(n.position.x), y: Math.round(n.position.y) }])))
        }
        onNodeDoubleClick={(_, node) => node.type === "table" && onOpenTable(node.id)}
        onNodeMouseEnter={handlers.onNodeMouseEnter}
        onNodeMouseLeave={handlers.onNodeMouseLeave}
        onEdgeMouseEnter={handlers.onEdgeMouseEnter}
        onEdgeMouseLeave={handlers.onEdgeMouseLeave}
        onSelectionChange={handlers.onSelectionChange}
        nodesConnectable={false}
        edgesFocusable={false}
        zoomOnDoubleClick={false}
        minZoom={0.1}
        maxZoom={2}
        // Room for the minimap and controls without covering cards after fitView.
        fitViewOptions={{ padding: 0.15 }}
        deleteKeyCode={null}
        selectionKeyCode="Shift"
        multiSelectionKeyCode={["Meta", "Control"]}
      >
        <Background variant={BackgroundVariant.Dots} gap={16} size={1.2} />
        <Controls showInteractive={false} position="bottom-left" />
        <MiniMap
          position="bottom-right"
          pannable
          zoomable
          ariaLabel="Graph overview"
          nodeBorderRadius={3}
          nodeClassName={(n) => (n.type === "ghost" ? "opacity-40" : "")}
        />
      </ReactFlow>
    </GraphHighlightContext.Provider>
  );
}
