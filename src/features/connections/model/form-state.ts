// Connection form state and the pure functions around it. No React here.
import type {
  AuthKind,
  ConnectionInput,
  DriverInfo,
  SavedConnection,
  SslMode,
  TargetInput,
  TargetKind,
} from "@/lib/ipc";
import { useSettingsStore } from "@/stores/settings";

import { type ConnectionColor, DEFAULT_COLOR, isConnectionColor } from "./colors";

export type SshSourceKind = "manual" | "config";
export type SshAuthKind = "password" | "key" | "agent" | "auto";

export interface FormState {
  /** Set when editing a saved connection. */
  id: string | null;
  driver: string;
  name: string;
  color: ConnectionColor;

  method: TargetKind;
  host: string;
  port: string; // kept as typed; parsed in toInput
  database: string;
  url: string;
  socketPath: string;
  filePath: string;

  authKind: AuthKind;
  user: string;
  password: string;
  savePassword: boolean;
  /** The keyring already has a password; an empty field keeps it. */
  passwordSaved: boolean;
  /** Client certificate auth: PEM files. */
  certPath: string;
  keyPath: string;

  sslMode: SslMode;
  /** Root CA file (PEM) for verify-ca / verify-full; empty = system roots. */
  caCert: string;

  sshEnabled: boolean;
  sshSource: SshSourceKind;
  /** Host alias from ~/.ssh/config. */
  sshAlias: string;
  sshHost: string;
  sshPort: string;
  sshUser: string;
  sshAuth: SshAuthKind;
  sshPassword: string;
  sshKeyPath: string;
  sshPassphrase: string;
  /** Save the SSH password / passphrase in the keyring. */
  sshSaveSecret: boolean;
  /** The keyring already has one; an empty field keeps it. */
  sshSecretSaved: boolean;

  options: Record<string, string>;
}

export type FormField = keyof FormState;
export type FormErrors = Partial<Record<FormField, string>>;

/** Auth kinds the form can render; others are hidden until it can. */
// TODO: token (cloud IAM) needs provider fields.
export const FORM_AUTH_KINDS: AuthKind[] = ["password", "passwordPrompt", "pgPass", "clientCert", "none"];

export const AUTH_LABELS: Record<AuthKind, string> = {
  password: "Password",
  passwordPrompt: "Ask for password every time",
  pgPass: "Password file (.pgpass)",
  none: "No authentication",
  clientCert: "Client certificate",
  token: "Cloud IAM token",
};

export const TARGET_LABELS: Record<TargetKind, string> = {
  host: "Host",
  url: "URL",
  socket: "Socket",
  file: "File",
};

export const SSL_LABELS: Record<SslMode, string> = {
  disable: "Disable",
  prefer: "Prefer",
  require: "Require",
  verifyCa: "Verify CA",
  verifyFull: "Verify full",
};

export const SSL_DESCRIPTIONS: Record<SslMode, string> = {
  disable: "No encryption.",
  prefer: "Encrypt when the server supports it; the certificate isn't checked.",
  require: "Always encrypt; the certificate isn't checked unless a CA file is set.",
  verifyCa: "Encrypt and check the certificate is signed by a trusted CA.",
  verifyFull: "Encrypt, check the CA and that the certificate matches the host name.",
};

export const SSH_AUTH_LABELS: Record<SshAuthKind, string> = {
  auto: "Keys from ~/.ssh, then agent",
  key: "Private key file",
  password: "Password",
  agent: "SSH agent",
};

const TARGET_ORDER: TargetKind[] = ["host", "url", "socket", "file"];

export function targetKinds(driver: DriverInfo): TargetKind[] {
  return TARGET_ORDER.filter((k) => driver.supportedTargets.includes(k));
}

export function authKinds(driver: DriverInfo): AuthKind[] {
  return FORM_AUTH_KINDS.filter((k) => driver.supportedAuth.includes(k));
}

export function needsUser(kind: AuthKind): boolean {
  return kind !== "none";
}

export function emptyForm(driver: DriverInfo): FormState {
  const options: Record<string, string> = {};
  for (const spec of driver.extraOptions) {
    if (spec.default != null) options[spec.key] = spec.default;
  }
  const defaults = useSettingsStore.getState().settings?.connection;
  return {
    id: null,
    driver: driver.id,
    name: "",
    color: DEFAULT_COLOR,
    method: targetKinds(driver)[0] ?? "host",
    host: "localhost",
    port: "",
    // TODO: per-driver default database once DriverInfo has one, same as the port.
    database: "postgres",
    url: "",
    socketPath: "",
    filePath: "",
    authKind: authKinds(driver)[0] ?? "none",
    user: "",
    password: "",
    savePassword: defaults?.savePasswordsByDefault ?? true,
    passwordSaved: false,
    certPath: "",
    keyPath: "",
    sslMode: driver.capabilities.tls ? (defaults?.defaultSslMode ?? "prefer") : "disable",
    caCert: "",
    sshEnabled: false,
    sshSource: "manual",
    sshAlias: "",
    sshHost: "",
    sshPort: "",
    sshUser: "",
    sshAuth: "auto",
    sshPassword: "",
    sshKeyPath: "",
    sshPassphrase: "",
    sshSaveSecret: defaults?.savePasswordsByDefault ?? true,
    sshSecretSaved: false,
    options,
  };
}

export function fromSaved(saved: SavedConnection, driver: DriverInfo): FormState {
  const form = emptyForm(driver);
  const t = saved.target;
  const auth = saved.auth;
  const tunnel = saved.tunnel;
  const sshAuth = tunnel?.auth;
  return {
    ...form,
    id: saved.id,
    name: saved.name,
    color: isConnectionColor(saved.color) ? saved.color : DEFAULT_COLOR,
    method: t.method,
    host: t.method === "host" ? t.host : "",
    port: t.method === "host" && t.port != null ? String(t.port) : "",
    database: t.method === "host" || t.method === "socket" ? (t.database ?? "") : "",
    url: t.method === "url" ? t.url : "",
    socketPath: t.method === "socket" ? t.path : "",
    filePath: t.method === "file" ? t.path : "",
    authKind: auth.kind,
    user: "user" in auth ? auth.user : "",
    password: "",
    savePassword: auth.kind === "password" ? auth.saved : form.savePassword,
    passwordSaved: auth.kind === "password" && auth.saved,
    certPath: auth.kind === "clientCert" ? auth.cert : "",
    keyPath: auth.kind === "clientCert" ? auth.key : "",
    sslMode: saved.tls.mode,
    caCert: saved.tls.caCert ?? "",
    sshEnabled: tunnel != null,
    sshSource: tunnel?.source?.kind === "config" ? "config" : "manual",
    sshAlias: tunnel?.source?.kind === "config" ? tunnel.source.alias : "",
    sshHost: tunnel?.host ?? "",
    sshPort: tunnel?.port != null ? String(tunnel.port) : "",
    sshUser: tunnel?.user ?? "",
    sshAuth: sshAuth ? (sshAuth.kind as SshAuthKind) : "auto",
    sshPassword: "",
    sshKeyPath: sshAuth?.kind === "key" ? sshAuth.path : "",
    sshPassphrase: "",
    sshSaveSecret:
      sshAuth?.kind === "password"
        ? sshAuth.saved
        : sshAuth?.kind === "key"
          ? sshAuth.passphraseSaved
          : form.sshSaveSecret,
    sshSecretSaved:
      (sshAuth?.kind === "password" && sshAuth.saved) || (sshAuth?.kind === "key" && sshAuth.passphraseSaved),
    options: { ...form.options, ...saved.options },
  };
}

function blankToNull(value: string): string | null {
  const v = value.trim();
  return v === "" ? null : v;
}

function portOrNull(value: string): number | null {
  const port = Number.parseInt(value, 10);
  return Number.isNaN(port) ? null : port;
}

export function toTarget(form: FormState): TargetInput {
  switch (form.method) {
    case "host":
      return {
        method: "host",
        host: form.host.trim(),
        port: portOrNull(form.port),
        database: blankToNull(form.database),
      };
    case "url":
      return { method: "url", url: form.url.trim() };
    case "socket":
      return { method: "socket", path: form.socketPath.trim(), database: blankToNull(form.database) };
    case "file":
      return { method: "file", path: form.filePath.trim() };
  }
}

/**
 * For saving, a password that isn't kept becomes "ask every time", so the
 * single save flag the backend has only applies to secrets the user wants
 * kept. For testing, the typed values are sent as they are.
 */
function toAuth(form: FormState, purpose: "test" | "save"): ConnectionInput["auth"] {
  const user = form.user.trim();
  switch (form.authKind) {
    case "password":
      if (purpose === "save" && !form.savePassword) return { kind: "passwordPrompt", user };
      return { kind: "password", user, password: form.password };
    case "passwordPrompt":
      return { kind: "passwordPrompt", user };
    case "pgPass":
      return { kind: "pgPass", user };
    case "clientCert":
      return { kind: "clientCert", user, cert: form.certPath.trim(), key: form.keyPath.trim() };
    default:
      return { kind: "none" };
  }
}

function toTunnel(form: FormState, purpose: "test" | "save"): ConnectionInput["tunnel"] {
  if (!form.sshEnabled) return null;
  const keep = purpose === "test" || form.sshSaveSecret;
  let auth: NonNullable<ConnectionInput["tunnel"]>["auth"];
  switch (form.sshAuth) {
    case "password":
      auth = keep ? { kind: "password", password: form.sshPassword } : { kind: "passwordPrompt" };
      break;
    case "key":
      auth = {
        kind: "key",
        path: form.sshKeyPath.trim(),
        // Empty with a saved one: the backend keeps it.
        passphrase: keep && (form.sshPassphrase || form.sshSecretSaved) ? form.sshPassphrase : null,
      };
      break;
    case "agent":
      auth = { kind: "agent" };
      break;
    case "auto":
      auth = { kind: "auto" };
      break;
  }
  const fromConfig = form.sshSource === "config";
  return {
    source: fromConfig ? { kind: "config", alias: form.sshAlias.trim() } : { kind: "manual" },
    // With a config alias, empty fields take the config's values.
    host: form.sshHost.trim(),
    port: portOrNull(form.sshPort),
    user: form.sshUser.trim(),
    auth,
  };
}

/** Saving keeps secrets only where the user asked; see `toAuth` / `toTunnel`. */
export function toInput(form: FormState, purpose: "test" | "save" = "save"): ConnectionInput {
  return {
    id: form.id,
    name: form.name.trim(),
    driver: form.driver,
    color: form.color,
    target: toTarget(form),
    auth: toAuth(form, purpose),
    tls: { mode: form.sslMode, caCert: blankToNull(form.caCert) },
    tunnel: toTunnel(form, purpose),
    options: Object.fromEntries(Object.entries(form.options).filter(([, v]) => v.trim() !== "")),
    // Unkept secrets were turned into prompts above, so this only keeps wanted ones.
    savePassword: purpose === "save",
  };
}

const DATABASE_REQUIRED = "Enter a database name.";

/** The database path segment of a connection URL, or "" when it has none. */
function urlDatabase(url: string): string {
  const rest = url.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "");
  const slash = rest.indexOf("/");
  if (slash === -1) return "";
  return rest.slice(slash + 1).split(/[?#]/)[0].trim();
}

function checkPort(value: string): string | undefined {
  const port = value.trim();
  if (!port) return undefined;
  if (!/^\d+$/.test(port)) return "Port must be a number.";
  if (Number(port) < 1 || Number(port) > 65535) return "Port must be between 1 and 65535.";
  return undefined;
}

/**
 * Field errors for the form. Testing doesn't need a name, so `purpose`
 * decides whether it is checked.
 */
export function validate(form: FormState, driver: DriverInfo, purpose: "test" | "save"): FormErrors {
  const errors: FormErrors = {};
  if (purpose === "save" && !form.name.trim()) errors.name = "Enter a name for this connection.";

  switch (form.method) {
    case "host": {
      if (!form.host.trim()) errors.host = "Enter a host name or IP address.";
      const port = checkPort(form.port);
      if (port) errors.port = port;
      if (!form.database.trim()) errors.database = DATABASE_REQUIRED;
      break;
    }
    case "url": {
      const url = form.url.trim();
      if (!url) errors.url = "Paste a connection URL.";
      else if (!/^[a-z][a-z0-9+.-]*:\/\//i.test(url)) errors.url = `URL must start with ${form.driver}://`;
      else if (!urlDatabase(url)) errors.url = `Add the database to the URL, e.g. ${form.driver}://host:port/mydb`;
      break;
    }
    case "socket":
      if (!form.socketPath.trim()) errors.socketPath = "Enter the socket directory.";
      if (!form.database.trim()) errors.database = DATABASE_REQUIRED;
      break;
    case "file":
      if (!form.filePath.trim()) errors.filePath = "Choose a database file.";
      break;
  }

  if (needsUser(form.authKind) && !form.user.trim()) errors.user = "Enter a user name.";
  if (form.authKind === "clientCert") {
    if (!form.certPath.trim()) errors.certPath = "Choose the client certificate (PEM).";
    if (!form.keyPath.trim()) errors.keyPath = "Choose the client key (PEM).";
  }

  if (form.sshEnabled) {
    if (form.sshSource === "config") {
      if (!form.sshAlias.trim()) errors.sshAlias = "Choose a host from ~/.ssh/config.";
    } else {
      if (!form.sshHost.trim()) errors.sshHost = "Enter the SSH host.";
      if (!form.sshUser.trim()) errors.sshUser = "Enter the SSH user.";
    }
    const port = checkPort(form.sshPort);
    if (port) errors.sshPort = port;
    if (form.sshAuth === "key" && !form.sshKeyPath.trim()) errors.sshKeyPath = "Choose the private key file.";
    if (form.method === "socket") errors.sshEnabled = "An SSH tunnel can't reach a Unix socket.";
  }

  for (const spec of driver.extraOptions) {
    if (spec.required && !form.options[spec.key]?.trim()) {
      errors.options = `${spec.label} is required.`;
    }
  }
  return errors;
}

export function hasErrors(errors: FormErrors): boolean {
  return Object.keys(errors).length > 0;
}
