import { useCallback, useEffect, useRef, useState } from "react";

import { api, type ConnectionInput, type ServerVersion } from "@/lib/ipc";
import { IpcError } from "@/lib/ipc-error";

export type TestState =
  | { status: "idle" }
  | { status: "testing" }
  | { status: "ok"; version: ServerVersion }
  | { status: "error"; error: IpcError };

/**
 * Runs `api.testConnection` and tracks its result. The result is cleared
 * whenever `resetKey` changes (i.e. the form was edited), so it never
 * describes values that are no longer in the form.
 */
export function useTestConnection(resetKey: unknown) {
  const [state, setState] = useState<TestState>({ status: "idle" });
  const run = useRef(0);

  useEffect(() => {
    run.current += 1; // drop any result still in flight
    setState({ status: "idle" });
  }, [resetKey]);

  const test = useCallback(async (input: ConnectionInput) => {
    const id = ++run.current;
    setState({ status: "testing" });
    try {
      const version = await api.testConnection(input);
      if (id === run.current) setState({ status: "ok", version });
    } catch (error) {
      if (id === run.current) setState({ status: "error", error: IpcError.from(error) });
    }
  }, []);

  return { state, test };
}
