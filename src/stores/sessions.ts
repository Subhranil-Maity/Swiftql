import { create } from "zustand";

import { api, type SessionInfo } from "@/lib/ipc";
import { useConnectionsStore } from "@/stores/connections";

interface SessionsState {
  sessions: Record<string, SessionInfo>;
  /** The session the workspace shows; null shows the welcome page. */
  activeSessionId: string | null;
  /** Saved connection currently being opened. */
  connectingId: string | null;
  /** Opens a saved connection and makes it active. Throws `IpcError`. */
  connect: (connectionId: string, password?: string) => Promise<SessionInfo>;
  disconnect: (sessionId: string) => Promise<void>;
}

export const useSessionsStore = create<SessionsState>()((set) => ({
  sessions: {},
  activeSessionId: null,
  connectingId: null,
  connect: async (connectionId, password) => {
    set({ connectingId: connectionId });
    try {
      const session = await api.connect(connectionId, password ?? null);
      set((s) => ({
        sessions: { ...s.sessions, [session.sessionId]: session },
        activeSessionId: session.sessionId,
      }));
      return session;
    } finally {
      set({ connectingId: null });
      // The backend recorded last-used and status either way.
      void useConnectionsStore.getState().load();
    }
  },
  disconnect: async (sessionId) => {
    try {
      await api.disconnect(sessionId);
    } finally {
      set((s) => {
        const { [sessionId]: _, ...sessions } = s.sessions;
        return {
          sessions,
          activeSessionId: s.activeSessionId === sessionId ? null : s.activeSessionId,
        };
      });
    }
  },
}));
