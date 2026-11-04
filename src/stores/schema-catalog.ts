// Each session's schemas, tables and columns, for autocomplete. In memory
// only; loaded when a console opens and reloaded after statements that change
// the structure.
import { create } from "zustand";

import { api, type Schema } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

export type Catalog = {
  schemas: Schema[];
  /** Where unqualified names resolve, first match wins. */
  searchPath: string[];
};

interface SchemaCatalogState {
  /** By session id; missing until the first load finishes. */
  catalogs: Record<string, Catalog>;
  /** Loads the session's catalog; a load already in flight is reused. */
  load: (sessionId: string) => Promise<void>;
  /** Loads it again, e.g. after CREATE TABLE. */
  refresh: (sessionId: string) => Promise<void>;
}

export const useSchemaCatalogStore = create<SchemaCatalogState>()((set, get) => {
  const inFlight = new Map<string, Promise<void>>();

  const fetchCatalog = (sessionId: string) => {
    const pending = inFlight.get(sessionId);
    if (pending) return pending;
    const request = (async () => {
      try {
        const snapshot = await api.introspect(sessionId, { scope: "catalog" });
        if (snapshot.kind !== "catalog") return;
        set((s) => ({ catalogs: { ...s.catalogs, [sessionId]: snapshot.data } }));
      } catch (error) {
        // Keep whatever was loaded before, e.g. while a transaction is aborted.
        // TODO: show that completions may be out of date
        console.warn(`loading the schema for autocomplete failed:`, errorMessage(error));
      } finally {
        inFlight.delete(sessionId);
      }
    })();
    inFlight.set(sessionId, request);
    return request;
  };

  return {
    catalogs: {},
    load: async (sessionId) => {
      if (get().catalogs[sessionId]) return;
      await fetchCatalog(sessionId);
    },
    refresh: async (sessionId) => {
      // A refresh asked for mid-load would read the old structure; wait and load again.
      await inFlight.get(sessionId);
      await fetchCatalog(sessionId);
    },
  };
});
