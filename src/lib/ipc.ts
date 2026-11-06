// The only way UI code talks to the backend. Wraps the generated
// `src/bindings.ts`: every `api.*` call resolves with the data or throws an
// `IpcError`, so callers use plain try/catch.
import * as bindings from "@/bindings";
import { commands } from "@/bindings";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

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
export type RunEvent = bindings.RunEvent_Serialize;
export type RunSummary = bindings.RunSummary_Serialize;
export type Notice = bindings.Notice_Serialize;
export type ColumnMeta = bindings.ColumnMeta_Serialize;
export type DbValue = bindings.DbValue_Serialize;
export type SchemaSnapshot = bindings.SchemaSnapshot_Serialize;
export type Schema = bindings.Schema_Serialize;
export type Table = bindings.Table_Serialize;
export type Column = bindings.Column_Serialize;
export type SshConfigHost = bindings.SshConfigHost_Serialize;
export type NotificationReceived = bindings.NotificationReceived_Serialize;
export type NotificationPayload = bindings.NotificationPayload;
export type ResultSelection = bindings.ResultSelection_Deserialize;
export type RowPage = bindings.RowPage_Serialize;
export type GeneralSettings = bindings.GeneralSettings_Serialize;
export type KeyringStatus = bindings.KeyringStatus_Serialize;

export type {
  Auth,
  AuthKind,
  Capabilities,
  ConfigError,
  ErrorKind,
  ExportOutcome,
  ResultSort,
  ResultState,
  BooleanDisplay,
  Density,
  HostKeyPolicy,
  KeywordCase,
  TimestampDisplay,
  TransactionMode,
  FkAction,
  ForeignKey,
  Index,
  FieldKind,
  HealthState,
  IntrospectScope,
  NoticeSeverity,
  ServerVersion,
  SessionInfo,
  SslMode,
  StoredAuth,
  TableKind,
  TargetKind,
  TxStatus,
} from "@/bindings";

export { IpcError } from "@/lib/ipc-error";
export { Channel } from "@tauri-apps/api/core";
/** The native "Open" dialog for picking a file; resolves with its path, or null. */
export async function pickFile(filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  const picked = await openDialog({ multiple: false, directory: false, filters });
  return typeof picked === "string" ? picked : null;
}

export type SaveFilter = { name: string; extensions: string[] };

/**
 * Asks where to save (the backend opens the "Save as" dialog), then writes
 * `bytes` there. Resolves with the path, or null if the dialog was cancelled.
 * The bytes go as a raw IPC body, not a JSON number array.
 */
export async function saveExport(
  defaultName: string,
  bytes: Uint8Array,
  filters: SaveFilter[],
): Promise<string | null> {
  const options = encodeURIComponent(JSON.stringify({ defaultName, filters }));
  try {
    return await invoke<string | null>("save_export", bytes, {
      headers: { "x-swiftql-save": options },
    });
  } catch (error) {
    throw IpcError.from(error);
  }
}
