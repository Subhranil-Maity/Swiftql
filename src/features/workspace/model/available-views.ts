// Views depend on what the session's driver can do: Notifications needs
// LISTEN / NOTIFY, which e.g. SQLite doesn't have.
import { useDriver } from "@/stores/drivers";
import type { WorkspaceView } from "@/stores/workspace-view";

/** Whether `view` makes sense for the driver; unknown drivers get every view. */
export function useViewAvailable(driverId: string): (view: WorkspaceView) => boolean {
  const caps = useDriver(driverId)?.capabilities;
  return (view) => view !== "notifications" || (caps?.listenNotify ?? true);
}
