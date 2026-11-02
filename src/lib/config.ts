// Typed bindings for the config commands in src-tauri/src/config.rs.
// Shapes mirror the camelCase serde types in crates/config-store.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Error returned by every config command. */
export interface ConfigError {
  kind: "io" | "json" | "sqlite" | "keyring" | "notFound" | "invalid";
  message: string;
}

/** ISO 8601 timestamp (chrono `DateTime<Utc>`). */
export type Timestamp = string;

// ---------- settings ----------

export type Theme = "light" | "dark" | "system";

export interface Settings {
  version: number;
  general: { theme: Theme };
  editor: {
    fontFamily: string;
    fontSize: number;
    tabSize: number;
    wordWrap: boolean;
  };
  query: {
    rowLimit: number;
    /** 0 = no timeout */
    queryTimeoutSecs: number;
    confirmDestructive: boolean;
  };
  connection: {
    connectTimeoutSecs: number;
    /** 0 = off */
    keepaliveSecs: number;
  };
}

type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends object ? DeepPartial<T[K]> : T[K] | null;
};

/** Deep-merged into the current settings. `null` resets a field to its default. */
export type SettingsPatch = DeepPartial<Omit<Settings, "version">>;

export const SETTINGS_CHANGED = "settings-changed";

// ---------- connections (driver-core types) ----------

export type Target =
  | { method: "url"; url: string }
  | { method: "host"; host: string; port?: number | null; database?: string | null }
  | { method: "socket"; path: string; database?: string | null }
  | { method: "file"; path: string };

export type TokenProvider =
  | { provider: "awsIam"; region: string }
  | { provider: "azureAd" };

export type SslMode = "disable" | "prefer" | "require" | "verifyCa" | "verifyFull";

export interface TlsConfig {
  mode?: SslMode;
  caCert?: string | null;
}

/** Auth as entered in the UI, secrets included. */
export type Auth =
  | { kind: "none" }
  | { kind: "password"; user: string; password: string }
  | { kind: "passwordPrompt"; user: string }
  | { kind: "pgPass"; user: string }
  | { kind: "clientCert"; user: string; cert: string; key: string }
  | { kind: "token"; user: string; provider: TokenProvider };

export type SshAuth =
  | { kind: "password"; password: string }
  | { kind: "key"; path: string; passphrase?: string | null }
  | { kind: "agent" };

export interface SshTunnel {
  host: string;
  port?: number | null;
  user: string;
  auth: SshAuth;
}

/**
 * A connection to save. Leave `id` out to create one. With `savePassword`,
 * an empty password or passphrase keeps the one already in the keyring.
 */
export interface ConnectionInput {
  id?: string | null;
  name: string;
  driver: string;
  folder?: string | null;
  color?: string | null;
  target: Target;
  auth: Auth;
  tls?: TlsConfig;
  tunnel?: SshTunnel | null;
  options?: Record<string, string>;
  savePassword?: boolean;
}

/** Auth as stored: no secrets, only whether one is saved in the keyring. */
export type StoredAuth =
  | { kind: "none" }
  | { kind: "password"; user: string; saved: boolean }
  | { kind: "passwordPrompt"; user: string }
  | { kind: "pgPass"; user: string }
  | { kind: "clientCert"; user: string; cert: string; key: string }
  | { kind: "token"; user: string; provider: TokenProvider };

export type StoredSshAuth =
  | { kind: "password"; saved: boolean }
  | { kind: "key"; path: string; passphraseSaved: boolean }
  | { kind: "agent" };

export interface StoredTunnel {
  host: string;
  port: number | null;
  user: string;
  auth: StoredSshAuth;
}

export interface ConnectionStatus {
  at: Timestamp;
  ok: boolean;
  /** driver-core `ErrorKind`, e.g. "auth", "network" */
  errorKind?: string | null;
  message?: string | null;
  serverVersion?: string | null;
}

export interface SavedConnection {
  id: string;
  name: string;
  driver: string;
  folder: string | null;
  color: string | null;
  sortOrder: number;
  target: Target;
  auth: StoredAuth;
  tls: Required<TlsConfig>;
  tunnel: StoredTunnel | null;
  options: Record<string, string>;
  createdAt: Timestamp;
  updatedAt: Timestamp;
  lastUsedAt: Timestamp | null;
  lastStatus: ConnectionStatus | null;
}

export type SecretKind = "dbPassword" | "sshPassword" | "sshPassphrase";

export interface KeyringStatus {
  available: boolean;
  message: string | null;
}

// ---------- commands ----------

export const getSettings = () => invoke<Settings>("get_settings");
export const updateSettings = (patch: SettingsPatch) =>
  invoke<Settings>("update_settings", { patch });
export const resetSettings = () => invoke<Settings>("reset_settings");
export const onSettingsChanged = (cb: (settings: Settings) => void): Promise<UnlistenFn> =>
  listen<Settings>(SETTINGS_CHANGED, (e) => cb(e.payload));

export const listConnections = () => invoke<SavedConnection[]>("list_connections");
export const recentConnections = (limit?: number) =>
  invoke<SavedConnection[]>("recent_connections", { limit });
export const getConnection = (id: string) => invoke<SavedConnection>("get_connection", { id });
export const saveConnection = (input: ConnectionInput) =>
  invoke<SavedConnection>("save_connection", { input });
export const deleteConnection = (id: string) => invoke<void>("delete_connection", { id });
export const touchConnection = (id: string) => invoke<void>("touch_connection", { id });
export const setConnectionStatus = (id: string, status: ConnectionStatus) =>
  invoke<void>("set_connection_status", { id, status });

export const clearConnectionSecret = (id: string, kind: SecretKind) =>
  invoke<SavedConnection>("clear_connection_secret", { id, kind });
export const keyringStatus = () => invoke<KeyringStatus>("keyring_status");
export const getAppState = <T = unknown>(key: string) =>
  invoke<T | null>("get_app_state", { key });
/** `null` removes the key. */
export const setAppState = (key: string, value: unknown) =>
  invoke<void>("set_app_state", { key, value });
