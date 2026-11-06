// The open session the workspace views work on, so deep components (a graph
// node's menu, a list row) don't need it passed down through every layer.
import { createContext, useContext } from "react";

export type Workspace = {
  /** Saved connection: keys what's kept across reconnects (tabs, graph layout). */
  connectionId: string;
  connectionName: string;
  sessionId: string;
  driverId: string;
  /** Connection colour id; see `colorVar`. */
  color: string | null | undefined;
};

export const WorkspaceContext = createContext<Workspace | null>(null);

export function useWorkspace(): Workspace {
  const workspace = useContext(WorkspaceContext);
  if (!workspace) throw new Error("useWorkspace must be used inside the workspace page");
  return workspace;
}
