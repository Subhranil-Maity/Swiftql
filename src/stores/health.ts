import { create } from "zustand";

import { api, type ConnectionHealth } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { useConnectionsStore } from "@/stores/connections";
import { useSettingsStore } from "@/stores/settings";

export type HealthEntry = ConnectionHealth | "checking";

interface HealthState {
  /** Live results by connection id. Never saved; gone on restart. */
  health: Record<string, HealthEntry>;
  /** Whether the startup check already ran this launch. */
  checkedRecent: boolean;
  /**
   * Checks the `connection.checkOnStartup` most recent connections, in
   * parallel. Runs once per launch, after connections and settings loaded.
   */
  checkRecent: () => void;
  /** Checks one connection, e.g. after it was saved or edited. */
  check: (id: string) => Promise<void>;
}

/** Bumped per check so a slow, stale result can't overwrite a newer one. */
const runs: Record<string, number> = {};

export const useHealthStore = create<HealthState>()((set, get) => ({
  health: {},
  checkedRecent: false,

  checkRecent: () => {
    if (get().checkedRecent) return;
    const { settings } = useSettingsStore.getState();
    const { connections, loaded } = useConnectionsStore.getState();
    if (!settings || !loaded) return;
    set({ checkedRecent: true });
    // The store keeps connections most recently used first.
    for (const c of connections.slice(0, settings.connection.checkOnStartup)) {
      void get().check(c.id);
    }
  },

  check: async (id) => {
    const run = (runs[id] = (runs[id] ?? 0) + 1);
    const put = (entry: HealthEntry | null) => {
      if (runs[id] !== run) return;
      set((s) => {
        const { [id]: _, ...rest } = s.health;
        return { health: entry ? { ...rest, [id]: entry } : rest };
      });
    };
    put("checking");
    try {
      put(await api.checkConnection(id));
    } catch (error) {
      // Couldn't load the connection (deleted, keyring error): no result,
      // the row falls back to its last status.
      console.warn(`health check for ${id} failed:`, errorMessage(error));
      put(null);
    }
  },
}));
