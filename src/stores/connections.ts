import { create } from "zustand";

import { api, type ConnectionInput, type SavedConnection } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

interface ConnectionsState {
  connections: SavedConnection[];
  loaded: boolean;
  error: string | null;
  load: () => Promise<void>;
  save: (input: ConnectionInput) => Promise<SavedConnection>;
  remove: (id: string) => Promise<void>;
}

/** Most recently used first; never-used ones after, in saved order. */
function byRecent(a: SavedConnection, b: SavedConnection): number {
  if (a.lastUsedAt && b.lastUsedAt) return b.lastUsedAt.localeCompare(a.lastUsedAt);
  if (a.lastUsedAt) return -1;
  if (b.lastUsedAt) return 1;
  return a.sortOrder - b.sortOrder;
}

export const useConnectionsStore = create<ConnectionsState>()((set) => ({
  connections: [],
  loaded: false,
  error: null,
  load: async () => {
    try {
      const connections = await api.listConnections();
      set({ connections: connections.sort(byRecent), loaded: true, error: null });
    } catch (error) {
      set({ loaded: true, error: errorMessage(error) });
    }
  },
  save: async (input) => {
    const saved = await api.saveConnection(input);
    set((s) => {
      const rest = s.connections.filter((c) => c.id !== saved.id);
      return { connections: [...rest, saved].sort(byRecent) };
    });
    return saved;
  },
  remove: async (id) => {
    await api.deleteConnection(id);
    set((s) => ({ connections: s.connections.filter((c) => c.id !== id) }));
  },
}));
