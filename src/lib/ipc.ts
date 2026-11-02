// The only way UI code talks to the backend. Wraps the generated
// `src/bindings.ts`: every `api.*` call resolves with the data or throws an
// `IpcError`, so callers use plain try/catch.
import * as bindings from "@/bindings";
import { commands } from "@/bindings";
import { IpcError } from "@/lib/ipc-error";

type Result<T, E> = { status: "ok"; data: T } | { status: "error"; error: E };

type Unwrapped<F> = F extends (...args: infer A) => Promise<Result<infer T, unknown>>
  ? (...args: A) => Promise<T>
  : never;

export type Api = { [K in keyof typeof commands]: Unwrapped<(typeof commands)[K]> };

async function unwrap<T>(pending: Promise<Result<T, unknown>>): Promise<T> {
  let result: Result<T, unknown>;
  try {
    result = await pending;
  } catch (error) {
    throw IpcError.from(error);
  }
  if (result.status === "error") throw IpcError.from(result.error);
  return result.data;
}

export const api = Object.fromEntries(
  Object.entries(commands).map(([name, command]) => [
    name,
    (...args: unknown[]) =>
      unwrap((command as (...a: unknown[]) => Promise<Result<unknown, unknown>>)(...args)),
  ]),
) as Api;

export const events = bindings.events;

// Values sent by Rust use the serialize shape; values we send use the
// deserialize shape (optional fields where serde has defaults).
export type SavedConnection = bindings.SavedConnection_Serialize;
export type ConnectionStatus = bindings.ConnectionStatus_Serialize;
export type ConnectionInput = bindings.ConnectionInput_Deserialize;
export type TargetInput = bindings.Target_Deserialize;
export type Target = bindings.Target_Serialize;
export type DriverInfo = bindings.DriverInfo_Serialize;
export type FieldSpec = bindings.FieldSpec_Serialize;
export type Settings = bindings.Settings_Serialize;
export type DbError = bindings.DbError_Serialize;
export type ConnectionHealth = bindings.ConnectionHealth_Serialize;
export type {
  Auth,
  AuthKind,
  Capabilities,
  ConfigError,
  ErrorKind,
  FieldKind,
  HealthState,
  ServerVersion,
  SessionInfo,
  SslMode,
  StoredAuth,
  TargetKind,
  TxStatus,
} from "@/bindings";

export { IpcError } from "@/lib/ipc-error";
