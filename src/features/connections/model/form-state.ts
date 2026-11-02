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

import { type ConnectionColor, DEFAULT_COLOR, isConnectionColor } from "./colors";

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

  // TODO: re-enable SSL modes once driver-postgres supports TLS (tls.rs)
  /** Always "disable" for now; see toInput. */
  sslMode: SslMode;

  options: Record<string, string>;
}

export type FormField = keyof FormState;
export type FormErrors = Partial<Record<FormField, string>>;

/** Auth kinds the form can render; others are hidden until it can. */
// TODO: clientCert (cert/key file pickers) and token (cloud IAM) need their own fields.
export const FORM_AUTH_KINDS: AuthKind[] = ["password", "passwordPrompt", "pgPass", "none"];

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
    savePassword: true,
    passwordSaved: false,
    sslMode: "disable",
    options,
  };
}

export function fromSaved(saved: SavedConnection, driver: DriverInfo): FormState {
  const form = emptyForm(driver);
  const t = saved.target;
  const auth = saved.auth;
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
    savePassword: auth.kind === "password" ? auth.saved : true,
    passwordSaved: auth.kind === "password" && auth.saved,
    // TODO: read saved.tls back once SSL modes are re-enabled.
    sslMode: "disable",
    options: { ...form.options, ...saved.options },
  };
}

function blankToNull(value: string): string | null {
  const v = value.trim();
  return v === "" ? null : v;
}

export function toTarget(form: FormState): TargetInput {
  switch (form.method) {
    case "host": {
      const port = Number.parseInt(form.port, 10);
      return {
        method: "host",
        host: form.host.trim(),
        port: Number.isNaN(port) ? null : port,
        database: blankToNull(form.database),
      };
    }
    case "url":
      return { method: "url", url: form.url.trim() };
    case "socket":
      return { method: "socket", path: form.socketPath.trim(), database: blankToNull(form.database) };
    case "file":
      return { method: "file", path: form.filePath.trim() };
  }
}

function toAuth(form: FormState): ConnectionInput["auth"] {
  const user = form.user.trim();
  switch (form.authKind) {
    case "password":
      return { kind: "password", user, password: form.password };
    case "passwordPrompt":
      return { kind: "passwordPrompt", user };
    case "pgPass":
      return { kind: "pgPass", user };
    default:
      return { kind: "none" };
  }
}

export function toInput(form: FormState): ConnectionInput {
  return {
    id: form.id,
    name: form.name.trim(),
    driver: form.driver,
    color: form.color,
    target: toTarget(form),
    auth: toAuth(form),
    // TODO: re-enable SSL modes once driver-postgres supports TLS (tls.rs)
    tls: { mode: "disable", caCert: null },
    options: Object.fromEntries(
      Object.entries(form.options).filter(([, v]) => v.trim() !== ""),
    ),
    savePassword: form.authKind === "password" && form.savePassword,
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

/**
 * Field errors for the form. Testing doesn't need a name, so `purpose`
 * decides whether it is checked.
 */
export function validate(
  form: FormState,
  driver: DriverInfo,
  purpose: "test" | "save",
): FormErrors {
  const errors: FormErrors = {};
  if (purpose === "save" && !form.name.trim()) errors.name = "Enter a name for this connection.";

  switch (form.method) {
    case "host": {
      if (!form.host.trim()) errors.host = "Enter a host name or IP address.";
      const port = form.port.trim();
      if (port && !/^\d+$/.test(port)) errors.port = "Port must be a number.";
      else if (port && (Number(port) < 1 || Number(port) > 65535))
        errors.port = "Port must be between 1 and 65535.";
      if (!form.database.trim()) errors.database = DATABASE_REQUIRED;
      break;
    }
    case "url": {
      const url = form.url.trim();
      if (!url) errors.url = "Paste a connection URL.";
      else if (!/^[a-z][a-z0-9+.-]*:\/\//i.test(url))
        errors.url = `URL must start with ${form.driver}://`;
      else if (!urlDatabase(url))
        errors.url = `Add the database to the URL, e.g. ${form.driver}://host:port/mydb`;
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
