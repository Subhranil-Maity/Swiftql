import { useCallback, useRef, useState } from "react";
import { toast } from "sonner";

import type { Secrets } from "@/features/connections/model/connect-flow";
import type { SavedConnection } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { useSessionsStore } from "@/stores/sessions";

import { useSecretFlow } from "./use-secret-flow";

/**
 * Connects to a saved connection. When the backend asks for a password, an
 * SSH secret or an SSH host key check, the matching prompt opens instead of
 * failing; other errors become a toast.
 */
export function useConnect() {
  const connectSession = useSessionsStore((s) => s.connect);
  const connectingId = useSessionsStore((s) => s.connectingId);
  const [connection, setConnection] = useState<SavedConnection | null>(null);
  const target = useRef<SavedConnection | null>(null);

  const attempt = useCallback(
    (secrets: Secrets) => connectSession(target.current!.id, secrets.password, secrets.sshSecret),
    [connectSession],
  );
  const onDone = useCallback(() => setConnection(null), []);
  const onError = useCallback((error: unknown) => {
    const name = target.current?.name ?? "the database";
    setConnection(null);
    toast.error(`Couldn't connect to ${name}`, { description: errorMessage(error), duration: 10_000 });
  }, []);
  const flow = useSecretFlow(attempt, onDone, onError);
  const { start, cancel } = flow;

  const connect = useCallback(
    async (c: SavedConnection) => {
      target.current = c;
      setConnection(c);
      start();
    },
    [start],
  );

  const cancelPrompt = useCallback(() => {
    cancel();
    setConnection(null);
  }, [cancel]);

  return {
    connect,
    connectingId,
    connection,
    prompt: flow.prompt,
    hostKey: flow.hostKey,
    submitPassword: flow.submitSecret,
    trustHost: () => void flow.trustHost(),
    cancelPrompt,
  };
}
