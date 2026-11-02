// Pure display helpers for connection targets. No secrets ever appear here.
import type { ServerVersion, SslMode, TargetInput } from "@/lib/ipc";

/** "16.4", or "8.0.36" when there is a patch number. */
export function formatVersion(v: ServerVersion): string {
  return v.patch > 0 ? `${v.major}.${v.minor}.${v.patch}` : `${v.major}.${v.minor}`;
}

/** Port a driver uses when none is given. */
// TODO: DriverInfo has no default port; move this to the driver once it does.
const DEFAULT_PORTS: Record<string, number> = { postgres: 5432 };

export function defaultPort(driverId: string): number | undefined {
  return DEFAULT_PORTS[driverId];
}

/** Removes `user:password@` from a URL. */
function stripCredentials(url: string): string {
  return url.replace(/^([a-z][a-z0-9+.-]*:\/\/)[^@/]*@/i, "$1");
}

/** Short "where is it" line: `db.acme.io:5432/orders`. */
export function formatTarget(target: TargetInput, driverId?: string): string {
  switch (target.method) {
    case "host": {
      const port = target.port ?? (driverId ? defaultPort(driverId) : undefined);
      const host = port ? `${target.host}:${port}` : target.host;
      return target.database ? `${host}/${target.database}` : host;
    }
    case "url":
      return stripCredentials(target.url).replace(/^[a-z][a-z0-9+.-]*:\/\//i, "");
    case "socket":
      return target.database ? `${target.path} (${target.database})` : target.path;
    case "file":
      return target.path;
  }
}

const SSL_PARAM: Record<SslMode, string> = {
  disable: "disable",
  prefer: "prefer",
  require: "require",
  verifyCa: "verify-ca",
  verifyFull: "verify-full",
};

/**
 * Read-only preview of the connection, e.g.
 * `postgres://alice@db.acme.io:5432/orders?sslmode=prefer`. Never includes
 * the password.
 */
// TODO: the URL scheme is the driver id; fine for postgres, revisit per driver.
export function toConnectionString(opts: {
  driverId: string;
  target: TargetInput;
  user?: string;
  sslMode?: SslMode;
}): string {
  const { driverId, target, user, sslMode } = opts;
  const scheme = `${driverId}://`;
  const userPart = user ? `${encodeURIComponent(user)}@` : "";
  const query = sslMode ? `?sslmode=${SSL_PARAM[sslMode]}` : "";

  switch (target.method) {
    case "url":
      return stripCredentials(target.url.trim()) || scheme;
    case "host": {
      const port = target.port ?? defaultPort(driverId);
      const host = target.host || "localhost";
      const db = target.database ? `/${encodeURIComponent(target.database)}` : "";
      return `${scheme}${userPart}${host}${port ? `:${port}` : ""}${db}${query}`;
    }
    case "socket": {
      const db = target.database ? `/${encodeURIComponent(target.database)}` : "/";
      return `${scheme}${userPart}${db}?host=${encodeURIComponent(target.path)}`;
    }
    case "file":
      return target.path;
  }
}
