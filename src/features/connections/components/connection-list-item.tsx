import { Spinner } from "@/components/ui/spinner";
import { formatTarget } from "@/lib/connection-format";
import type { SavedConnection } from "@/lib/ipc";
import { useHealthStore } from "@/stores/health";

import { colorVar } from "../model/colors";
import { ConnectionActionsMenu } from "./connection-actions-menu";
import { DriverLogo } from "./driver-logo";
import { HealthDot } from "./health-dot";

/**
 * One saved connection. Its colour runs down the left edge; click or Enter
 * connects. Status on the right: a spinner while connecting, else the live
 * health check when there is one, else the last connect (server version, or
 * a red dot after a failure).
 */
export function ConnectionListItem({
  connection,
  connecting,
  disabled,
  onConnect,
  onEdit,
  onDelete,
}: {
  connection: SavedConnection;
  connecting: boolean;
  disabled: boolean;
  onConnect: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const status = connection.lastStatus;
  const health = useHealthStore((s) => s.health[connection.id]);
  const target = formatTarget(connection.target, connection.driver);

  return (
    <li className="group/row relative rounded-md hover:bg-muted/60 has-disabled:hover:bg-transparent">
      <span
        aria-hidden
        className="absolute inset-y-1.5 left-0 w-[3px] rounded-full"
        style={{ background: colorVar(connection.color) }}
      />
      <button
        type="button"
        onClick={onConnect}
        disabled={disabled}
        aria-busy={connecting}
        className="grid w-full grid-cols-[auto_minmax(0,11rem)_minmax(0,1fr)_3rem] items-center gap-3 rounded-md py-2 pr-10 pl-4 text-left outline-none focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-default disabled:opacity-50 aria-busy:opacity-100"
      >
        <DriverLogo driverId={connection.driver} />
        <span className="truncate font-medium">{connection.name}</span>
        <span className="truncate font-mono text-xs text-muted-foreground">{target}</span>
        <span className="flex justify-end">
          {connecting ? (
            <Spinner aria-label={`Connecting to ${connection.name}`} />
          ) : health ? (
            <HealthDot entry={health} name={connection.name} />
          ) : status?.ok ? (
            <span className="font-mono text-xs text-muted-foreground">
              <span className="sr-only">Server version </span>
              {status.serverVersion}
            </span>
          ) : status ? (
            <span title={status.message ?? undefined} className="flex">
              <span aria-hidden className="size-2 rounded-full bg-destructive" />
              <span className="sr-only">Last connection failed: {status.message}</span>
            </span>
          ) : null}
        </span>
      </button>
      <ConnectionActionsMenu
        name={connection.name}
        onEdit={onEdit}
        onDelete={onDelete}
        disabled={disabled}
        className="absolute top-1/2 right-1 -translate-y-1/2"
      />
    </li>
  );
}
