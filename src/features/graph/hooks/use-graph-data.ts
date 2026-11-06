import { useCallback, useEffect, useMemo } from "react";

import { buildGraph } from "@/features/graph/model/build-graph";
import { useGraphLayoutStore } from "@/features/graph/stores/graph-layout";
import type { SchemaModel } from "@/stores/schema-model";

const NO_POSITIONS = {};

/** Schemas shown until the user picks: the search path's, else every schema that has tables. */
function defaultSchemas(model: SchemaModel): string[] {
  const withTables = model.schemas.filter((s) => (s.tables?.length ?? 0) > 0).map((s) => s.name);
  const onPath = withTables.filter((name) => model.searchPath.includes(name));
  return onPath.length > 0 ? onPath : withTables;
}

/** The model as nodes and edges for the chosen schemas, plus the saved layout. */
export function useGraphData(model: SchemaModel | undefined, connectionId: string) {
  const layout = useGraphLayoutStore((s) => s.byConnection[connectionId]);
  const load = useGraphLayoutStore((s) => s.load);
  const saveSchemas = useGraphLayoutStore((s) => s.setSchemas);

  useEffect(() => {
    void load(connectionId);
  }, [load, connectionId]);

  const schemas = useMemo(() => model?.schemas.map((s) => s.name) ?? [], [model]);
  const selected = useMemo<ReadonlySet<string>>(() => {
    if (!model) return new Set();
    // Saved schemas that were dropped since are ignored.
    const saved = layout?.schemas?.filter((s) => schemas.includes(s));
    return new Set(saved ?? defaultSchemas(model));
  }, [model, layout?.schemas, schemas]);

  const graph = useMemo(
    () => (model ? buildGraph(model, selected) : { nodes: [], edges: [] }),
    [model, selected],
  );

  const setSchemas = useCallback(
    (next: Iterable<string>) => saveSchemas(connectionId, [...next]),
    [saveSchemas, connectionId],
  );

  return {
    /** The saved layout has been read; positions are final. */
    loaded: layout !== undefined,
    graph,
    schemas,
    selected,
    setSchemas,
    positions: layout?.positions ?? NO_POSITIONS,
  };
}
