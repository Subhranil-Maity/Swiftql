import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ReactFlowProvider, useReactFlow } from "@xyflow/react";

import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { GraphCanvas } from "@/features/graph/components/graph-canvas";
import { GraphToolbar } from "@/features/graph/components/graph-toolbar";
import { useAutoLayout } from "@/features/graph/hooks/use-auto-layout";
import { useGraphData } from "@/features/graph/hooks/use-graph-data";
import { useGraphExport } from "@/features/graph/hooks/use-graph-export";
import { useGraphLayoutStore } from "@/features/graph/stores/graph-layout";
import { ModelLoadState } from "@/features/tables/components/model-load-state";
import { TableContextMenu } from "@/features/tables/components/table-context-menu";
import { TableDetailsSheet } from "@/features/tables/components/table-details-sheet";
import { useOpenTableData } from "@/features/tables/hooks/use-open-table-data";
import { useSchemaModel } from "@/features/tables/hooks/use-schema-model";
import { type TableEntry, tableEntries } from "@/features/tables/model/table-entry";
import { useWorkspace } from "@/features/workspace/model/workspace-context";
import { useWorkspaceViewStore } from "@/stores/workspace-view";

/** The ER diagram: every table in the chosen schemas, linked by foreign keys. */
export function GraphView({ active }: { active: boolean }) {
  return (
    <ReactFlowProvider>
      <GraphWorkspace active={active} />
    </ReactFlowProvider>
  );
}

function GraphWorkspace({ active }: { active: boolean }) {
  const { sessionId, connectionId, connectionName } = useWorkspace();
  const { model, status, refresh } = useSchemaModel(sessionId);
  const data = useGraphData(model, connectionId);
  const layout = useAutoLayout(data.graph, data.positions, data.loaded && model !== undefined);
  const { fitView, setNodes } = useReactFlow();
  const { openTableData } = useOpenTableData();
  const setPositions = useGraphLayoutStore((s) => s.setPositions);
  const clearPositions = useGraphLayoutStore((s) => s.clearPositions);
  const canvasRef = useRef<HTMLDivElement>(null);
  const exportAs = useGraphExport(canvasRef, `${connectionName} diagram`);

  const entries = useMemo(() => (model ? tableEntries(model) : []), [model]);
  const byKey = useMemo(() => new Map(entries.map((e) => [e.key, e] as const)), [entries]);
  const [detailsKey, setDetailsKey] = useState<string | null>(null);
  const [finderOpen, setFinderOpen] = useState(false);
  const shownNodes = layout.nodes.filter((n) => !n.hidden);

  // Fit the whole diagram when it first appears and when the schemas change,
  // not after every refresh (that would throw away the user's zoom).
  const schemasKey = [...data.selected].join("\u0000");
  const fittedFor = useRef<string | null>(null);
  useEffect(() => {
    if (!layout.ready || fittedFor.current === schemasKey) return;
    fittedFor.current = schemasKey;
    requestAnimationFrame(() => void fitView({ padding: 0.15 }));
  }, [layout.ready, schemasKey, fitView]);

  // A table to pan to once it's on the canvas: from Find, or "Show in graph" in Tables.
  const [pendingFocus, setPendingFocus] = useState<string | null>(null);
  const focusTable = useCallback(
    (entry: TableEntry) => {
      if (!data.selected.has(entry.schema)) data.setSchemas([...data.selected, entry.schema]);
      setPendingFocus(entry.key);
    },
    [data],
  );
  useEffect(() => {
    if (!pendingFocus || !layout.ready) return;
    if (!layout.nodes.some((n) => n.id === pendingFocus && !n.hidden)) return;
    const key = pendingFocus;
    setPendingFocus(null);
    // The schema change above would fit the whole diagram; this pan should win.
    fittedFor.current = schemasKey;
    setNodes((nodes) => nodes.map((n) => ({ ...n, selected: n.id === key })));
    requestAnimationFrame(() => void fitView({ nodes: [{ id: key }], duration: 300, maxZoom: 1.1, padding: 0.6 }));
  }, [pendingFocus, layout.ready, layout.nodes, fitView, setNodes, schemasKey]);

  const graphFocus = useWorkspaceViewStore((s) => s.graphFocus[connectionId]);
  const clearGraphFocus = useWorkspaceViewStore((s) => s.clearGraphFocus);
  useEffect(() => {
    if (!graphFocus || !model || !data.loaded) return;
    clearGraphFocus(connectionId, graphFocus.nonce);
    const entry = byKey.get(graphFocus.key);
    if (entry) focusTable(entry);
  }, [graphFocus, model, data.loaded, byKey, focusTable, clearGraphFocus, connectionId]);

  // Ctrl/Cmd+F opens Find while the graph is showing.
  useEffect(() => {
    if (!active) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "f") {
        event.preventDefault();
        setFinderOpen(true);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [active]);

  const relayout = () => {
    clearPositions(
      connectionId,
      layout.nodes.map((n) => n.id),
    );
    fittedFor.current = null;
  };

  if (!model) {
    return <ModelLoadState error={status.loading ? null : status.error} onRetry={refresh} />;
  }

  let body;
  if (entries.length === 0) {
    body = (
      <Empty>
        <EmptyHeader>
          <EmptyTitle>No tables to draw</EmptyTitle>
          <EmptyDescription>Create tables in the console and they show up here.</EmptyDescription>
        </EmptyHeader>
      </Empty>
    );
  } else if (data.graph.nodes.length === 0) {
    body = (
      <Empty>
        <EmptyHeader>
          <EmptyTitle>No tables in {data.selected.size === 1 ? [...data.selected][0] : "these schemas"}</EmptyTitle>
          <EmptyDescription>Pick other schemas with the Schemas menu above.</EmptyDescription>
        </EmptyHeader>
      </Empty>
    );
  } else {
    body = (
      <TableContextMenu
        resolve={(target) => {
          const id = target.closest(".react-flow__node")?.getAttribute("data-id");
          return id ? byKey.get(id) : undefined;
        }}
        onViewDetails={(entry) => setDetailsKey(entry.key)}
        showInGraph={false}
      >
        <div ref={canvasRef} className="relative min-h-0 flex-1 bg-background">
          <GraphCanvas
            nodes={layout.nodes}
            edges={data.graph.edges}
            onDragged={(positions) => setPositions(connectionId, positions)}
            onOpenTable={(key) => {
              const entry = byKey.get(key);
              if (entry) void openTableData(entry.schema, entry.table.name);
            }}
          />
          {layout.error && (
            <p className="absolute top-3 left-1/2 -translate-x-1/2 rounded-md border bg-popover px-3 py-1.5 text-xs text-muted-foreground shadow-sm">
              Auto-layout failed: {layout.error}
            </p>
          )}
        </div>
      </TableContextMenu>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <GraphToolbar
        schemas={data.schemas}
        selected={data.selected}
        searchPath={model.searchPath}
        onSchemasChange={data.setSchemas}
        entries={entries}
        finderOpen={finderOpen}
        onFinderOpenChange={setFinderOpen}
        onFind={focusTable}
        onRelayout={relayout}
        onExport={(format) => void exportAs(format)}
        empty={shownNodes.length === 0}
      />
      {body}
      <TableDetailsSheet
        entry={detailsKey ? (byKey.get(detailsKey) ?? null) : null}
        model={model}
        onNavigate={setDetailsKey}
        onClose={() => setDetailsKey(null)}
        showInGraph={false}
      />
    </div>
  );
}
