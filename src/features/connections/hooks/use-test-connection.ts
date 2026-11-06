import { useCallback, useEffect, useRef, useState } from "react";

import type { Secrets } from "@/features/connections/model/connect-flow";
import { api, type ConnectionInput, type ServerVersion } from "@/lib/ipc";
import { IpcError } from "@/lib/ipc-error";

import { useSecretFlow } from "./use-secret-flow";

export type TestState =
  | { status: "idle" }
  | { status: "testing" }
  | { status: "ok"; version: ServerVersion }
  | { status: "error"; error: IpcError };

/**
 * Runs `api.testConnection` and tracks its result, asking for a one-off
 * password / SSH secret or an SSH host key check on the way. The result is
 * cleared whenever `resetKey` changes (i.e. the form was edited), so it
 * never describes values that are no longer in the form.
 */
export function useTestConnection(resetKey: unknown) {
  const [state, setState] = useState<TestState>({ status: "idle" });
  const run = useRef(0);
  const input = useRef<ConnectionInput | null>(null);

  const attempt = useCallback(
    (secrets: Secrets) =>
      api.testConnection(input.current!, secrets.password ?? null, secrets.sshSecret ?? null),
    [],
  );
  const onDone = useCallback((version: ServerVersion) => {
    if (input.current) setState({ status: "ok", version });
  }, []);
  const onError = useCallback((error: unknown) => {
    if (input.current) setState({ status: "error", error: IpcError.from(error) });
  }, []);
  const flow = useSecretFlow(attempt, onDone, onError);
  const { start, cancel } = flow;

  useEffect(() => {
    run.current += 1;
    input.current = null; // drop any result still in flight
    cancel();
    setState({ status: "idle" });
  }, [resetKey, cancel]);

  const test = useCallback(
    (next: ConnectionInput) => {
      input.current = next;
      setState({ status: "testing" });
      start();
    },
    [start],
  );

  const cancelPrompt = useCallback(() => {
    cancel();
    input.current = null;
    setState({ status: "idle" });
  }, [cancel]);

  return {
    state,
    test,
    prompt: flow.prompt,
    hostKey: flow.hostKey,
    submitSecret: flow.submitSecret,
    trustHost: () => void flow.trustHost(),
    cancelPrompt,
  };
}
