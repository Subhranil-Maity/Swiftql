import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
import { DriverLogo } from "@/features/connections/components/driver-logo";
import { colorVar } from "@/features/connections/model/colors";
import { formatVersion } from "@/lib/connection-format";
import type { SessionInfo } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { useConnectionsStore } from "@/stores/connections";
import { useSessionsStore } from "@/stores/sessions";

/** Placeholder workspace for one open session. */
export function WorkspacePage({ session }: { session: SessionInfo }) {
  const connection = useConnectionsStore((s) =>
    s.connections.find((c) => c.id === session.connectionId),
  );
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
    <main className="flex min-h-svh flex-col bg-background text-foreground">
      <header className="flex items-center gap-3 px-4 py-2.5">
        <DriverLogo driverId={session.driverId} />
        <h1 className="truncate font-medium">{name}</h1>
        <span className="font-mono text-xs text-muted-foreground">
          {session.serverVersion.product} {formatVersion(session.serverVersion)}
        </span>
        <Button
          variant="outline"
          size="sm"
          className="ml-auto"
          onClick={() => void onDisconnect()}
          disabled={leaving}
        >
          {leaving && <Spinner data-icon="inline-start" />}
          Disconnect
        </Button>
      </header>
      {/* The connection's colour stays in view while you work. */}
      <div aria-hidden className="h-0.5 w-full" style={{ background: colorVar(connection?.color) }} />

      {/* TODO: query editor and results grid */}
      <Empty className="flex-1">
        <EmptyHeader>
          <EmptyTitle>Query editor coming next</EmptyTitle>
          <EmptyDescription>You're connected. Disconnect to go back to your connections.</EmptyDescription>
        </EmptyHeader>
      </Empty>
    </main>
  );
}
