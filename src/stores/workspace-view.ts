// Which main view the workspace shows, per saved connection. In memory only:
// a reconnect starts in the console.
import { create } from "zustand";

export type WorkspaceView = "console" | "tables" | "graph";

/** A table the graph should pan to once it's showing. */
export type GraphFocus = { key: string; nonce: number };

interface WorkspaceViewState {
  byConnection: Record<string, WorkspaceView>;
  /** Set by "Show in graph"; the graph view clears it after panning. */
  graphFocus: Record<string, GraphFocus>;
  setView: (connectionId: string, view: WorkspaceView) => void;
  /** Switches to the graph and pans to the table (`key` is its qualified name). */
  showInGraph: (connectionId: string, key: string) => void;
  clearGraphFocus: (connectionId: string, nonce: number) => void;
}

let nextNonce = 1;

export const useWorkspaceViewStore = create<WorkspaceViewState>()((set) => ({
  byConnection: {},
  graphFocus: {},
  setView: (connectionId, view) =>
    set((s) => ({ byConnection: { ...s.byConnection, [connectionId]: view } })),
  showInGraph: (connectionId, key) =>
    set((s) => ({
      byConnection: { ...s.byConnection, [connectionId]: "graph" },
      graphFocus: { ...s.graphFocus, [connectionId]: { key, nonce: nextNonce++ } },
    })),
  clearGraphFocus: (connectionId, nonce) =>
    set((s) => {
      if (s.graphFocus[connectionId]?.nonce !== nonce) return s;
      const { [connectionId]: _, ...graphFocus } = s.graphFocus;
      return { graphFocus };
    }),
}));

/** The connection's view; the console until another is picked. */
export function useWorkspaceView(connectionId: string): WorkspaceView {
  return useWorkspaceViewStore((s) => s.byConnection[connectionId] ?? "console");
}
