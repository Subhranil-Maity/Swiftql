import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { DriverLogo } from "@/features/connections/components/driver-logo";
import { colorVar } from "@/features/connections/model/colors";
import { formatVersion } from "@/lib/connection-format";
import type { SavedConnection, SessionInfo } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { useSessionsStore } from "@/stores/sessions";

import { TransactionStatus } from "./transaction-status";

/** Top bar: what you're connected to, the transaction status, and Disconnect. */
export function WorkspaceHeader({
  session,
  connection,
}: {
  session: SessionInfo;
  connection: SavedConnection | undefined;
}) {
  const disconnect = useSessionsStore((s) => s.disconnect);
  const [leaving, setLeaving] = useState(false);
  const name = connection?.name ?? "Connection";

  const onDisconnect = async () => {
    setLeaving(true);
    try {
      await disconnect(session.sessionId);
    } catch (error) {
      // The session is dropped from the UI either way.
      toast.error(`Couldn't close ${name} cleanly`, { description: errorMessage(error) });
    }
  };

  return (
    <div className="shrink-0">
      {/* Same height as the sidebar header so the two line up. */}
      <header className="flex h-12 items-center gap-3 px-4">
        <DriverLogo driverId={session.driverId} />
        <h1 className="truncate font-medium">{name}</h1>
        <span className="font-mono text-xs text-muted-foreground">
          {session.serverVersion.product} {formatVersion(session.serverVersion)}
        </span>
        <div className="ml-auto flex items-center gap-3">
          <TransactionStatus session={session} />
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void onDisconnect()}
          disabled={leaving}
        >
          {leaving && <Spinner data-icon="inline-start" />}
          Disconnect
        </Button>
      </header>
      {/* The connection's colour stays in view while you work. */}
      <div aria-hidden className="h-0.5 w-full" style={{ background: colorVar(connection?.color) }} />
    </div>
  );
}
