// Per saved connection: where the user dragged tables to, and which schemas
// the graph shows. Saved to app state (debounced, like console-tabs.ts), so
// both survive a restart.
import { create } from "zustand";

import type { XY } from "@/features/graph/model/layout";
import { api } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

export type GraphLayout = {
  /** Dragged positions by table key; they win over auto-layout. */
  positions: Record<string, XY>;
  /** Schemas shown; null until the user picks (then the search path is used). */
  schemas: string[] | null;
};

const storageKey = (connectionId: string) => `graph.layout.${connectionId}`;
const SAVE_DELAY_MS = 500;
const pendingSaves = new Map<string, ReturnType<typeof setTimeout>>();
const EMPTY: GraphLayout = { positions: {}, schemas: null };

function parseLayout(value: unknown): GraphLayout {
  if (typeof value !== "object" || value === null) return EMPTY;
  const v = value as Record<string, unknown>;
  const positions: Record<string, XY> = {};
  if (typeof v.positions === "object" && v.positions !== null) {
    for (const [key, p] of Object.entries(v.positions as Record<string, unknown>)) {
      const xy = p as Partial<XY> | null;
      if (xy && Number.isFinite(xy.x) && Number.isFinite(xy.y)) positions[key] = { x: xy.x!, y: xy.y! };
    }
  }
  const schemas =
    Array.isArray(v.schemas) && v.schemas.every((s) => typeof s === "string") ? (v.schemas as string[]) : null;
  return { positions, schemas };
}

interface GraphLayoutState {
  byConnection: Record<string, GraphLayout>;
  /** Reads the saved layout once per connection. */
  load: (connectionId: string) => Promise<void>;
  setPositions: (connectionId: string, positions: Record<string, XY>) => void;
  /** Forgets dragged positions, e.g. for Re-layout. */
  clearPositions: (connectionId: string, keys?: string[]) => void;
  setSchemas: (connectionId: string, schemas: string[]) => void;
}

export const useGraphLayoutStore = create<GraphLayoutState>()((set, get) => {
  const loading = new Set<string>();

  const scheduleSave = (connectionId: string) => {
    clearTimeout(pendingSaves.get(connectionId));
    pendingSaves.set(
      connectionId,
      setTimeout(() => {
        pendingSaves.delete(connectionId);
        const layout = get().byConnection[connectionId];
        if (!layout) return;
        // TODO: surface save failures in the UI instead of only logging them
        api.setAppState(storageKey(connectionId), layout).catch((error: unknown) => {
          console.warn(`saving the graph layout for ${connectionId} failed:`, errorMessage(error));
        });
      }, SAVE_DELAY_MS),
    );
  };

  const update = (connectionId: string, change: (layout: GraphLayout) => GraphLayout) => {
    const current = get().byConnection[connectionId] ?? EMPTY;
    set((s) => ({ byConnection: { ...s.byConnection, [connectionId]: change(current) } }));
    scheduleSave(connectionId);
  };

  return {
    byConnection: {},
    load: async (connectionId) => {
      if (get().byConnection[connectionId] || loading.has(connectionId)) return;
      loading.add(connectionId);
      let layout = EMPTY;
      try {
        layout = parseLayout(await api.getAppState(storageKey(connectionId)));
      } catch (error) {
        console.warn(`loading the graph layout for ${connectionId} failed:`, errorMessage(error));
      } finally {
        loading.delete(connectionId);
      }
      // A change made while loading (unlikely) wins over the saved copy.
      set((s) => (s.byConnection[connectionId] ? s : { byConnection: { ...s.byConnection, [connectionId]: layout } }));
    },
    setPositions: (connectionId, positions) =>
      update(connectionId, (l) => ({ ...l, positions: { ...l.positions, ...positions } })),
    clearPositions: (connectionId, keys) =>
      update(connectionId, (l) => {
        if (!keys) return { ...l, positions: {} };
        const positions = { ...l.positions };
        for (const key of keys) delete positions[key];
        return { ...l, positions };
      }),
    setSchemas: (connectionId, schemas) => update(connectionId, (l) => ({ ...l, schemas })),
  };
});
