// Each session's full schema model (keys, indexes, comments, sizes) for the
// Tables and Graph views. Same pattern as schema-catalog.ts, which stays the
// lighter load autocomplete needs. In memory only.
import { create } from "zustand";

import { api, type Schema } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

export type SchemaModel = {
  schemas: Schema[];
  searchPath: string[];
};

export type ModelStatus = {
  loading: boolean;
  /** The last load's failure; cleared by the next success. */
  error: string | null;
};

interface SchemaModelState {
  /** By session id; missing until the first load finishes. */
  models: Record<string, SchemaModel>;
  status: Record<string, ModelStatus>;
  /** Loads the session's model once; a load already in flight is reused. */
  load: (sessionId: string) => Promise<void>;
  /** Loads it again, e.g. the Refresh button. */
  refresh: (sessionId: string) => Promise<void>;
  /** Reloads only if a view has loaded it, e.g. after CREATE TABLE in the console. */
  refreshIfLoaded: (sessionId: string) => Promise<void>;
}

export const useSchemaModelStore = create<SchemaModelState>()((set, get) => {
  const inFlight = new Map<string, Promise<void>>();

  const setStatus = (sessionId: string, status: ModelStatus) =>
    set((s) => ({ status: { ...s.status, [sessionId]: status } }));

  const fetchModel = (sessionId: string) => {
    const pending = inFlight.get(sessionId);
    if (pending) return pending;
    const request = (async () => {
      setStatus(sessionId, { loading: true, error: get().status[sessionId]?.error ?? null });
      try {
        const snapshot = await api.introspect(sessionId, { scope: "model" });
        if (snapshot.kind !== "model") throw new Error(`expected a model, got ${snapshot.kind}`);
        set((s) => ({
          models: { ...s.models, [sessionId]: snapshot.data },
          status: { ...s.status, [sessionId]: { loading: false, error: null } },
        }));
      } catch (error) {
        // Keep whatever was loaded before, e.g. while a transaction is aborted.
        setStatus(sessionId, { loading: false, error: errorMessage(error) });
      } finally {
        inFlight.delete(sessionId);
      }
    })();
    inFlight.set(sessionId, request);
    return request;
  };

  return {
    models: {},
    status: {},
    load: async (sessionId) => {
      if (get().models[sessionId]) return;
      await fetchModel(sessionId);
    },
    refresh: async (sessionId) => {
      // A refresh asked for mid-load would read the old structure; wait and load again.
      await inFlight.get(sessionId);
      await fetchModel(sessionId);
    },
    refreshIfLoaded: async (sessionId) => {
      if (!get().models[sessionId] && !inFlight.has(sessionId)) return;
      await get().refresh(sessionId);
    },
  };
});
