import { CheckIcon, CircleAlertIcon, XIcon } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Spinner } from "@/components/ui/spinner";
import { formatVersion } from "@/lib/connection-format";
import type { IpcError } from "@/lib/ipc-error";

import type { TestState } from "../hooks/use-test-connection";

/** One-line result next to the Test connection button. Announced politely. */
export function TestConnectionResult({ state }: { state: TestState }) {
  return (
    <p aria-live="polite" className="flex min-w-0 items-center gap-1.5">
      {state.status === "testing" && (
        <>
          <Spinner aria-hidden className="text-muted-foreground" />
          <span className="text-muted-foreground">Testing</span>
        </>
      )}
      {state.status === "ok" && (
        <>
          <CheckIcon aria-hidden className="size-4 shrink-0 text-success" />
          <span className="truncate">
            Connected to {state.version.product}{" "}
            <span className="font-mono">{formatVersion(state.version)}</span>
          </span>
        </>
      )}
      {state.status === "error" && (
        <>
          <XIcon aria-hidden className="size-4 shrink-0 text-destructive" />
          <span className="text-destructive">Connection failed</span>
        </>
      )}
    </p>
  );
}

/** Full driver error: message, then hint and detail when the driver gave them. */
export function ConnectionErrorAlert({ error }: { error: IpcError }) {
  return (
    <Alert variant="destructive">
      <CircleAlertIcon />
      <AlertTitle className="break-words">{error.message}</AlertTitle>
      {(error.hint || error.detail) && (
        <AlertDescription className="flex flex-col gap-1">
          {error.hint && <span>{error.hint}</span>}
          {error.detail && <span className="font-mono text-xs break-words">{error.detail}</span>}
        </AlertDescription>
      )}
    </Alert>
  );
}
