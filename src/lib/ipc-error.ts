import type { ConfigError, DbError } from "@/bindings";

/** `DbError.code` when a saved connection has no password to connect with. */
export const PASSWORD_REQUIRED = "PASSWORD_REQUIRED";

/**
 * Every failed backend call throws this. It wraps a `DbError`, a `ConfigError`
 * or a framework failure (unknown command, bad argument), which Tauri reports
 * as a plain string.
 */
export class IpcError extends Error {
  readonly kind: string;
  readonly code: string | null;
  readonly hint: string | null;
  readonly detail: string | null;

  constructor(
    message: string,
    opts: { kind?: string; code?: string | null; hint?: string | null; detail?: string | null } = {},
  ) {
    super(message);
    this.name = "IpcError";
    this.kind = opts.kind ?? "internal";
    this.code = opts.code ?? null;
    this.hint = opts.hint ?? null;
    this.detail = opts.detail ?? null;
  }

  static from(error: unknown): IpcError {
    if (error instanceof IpcError) return error;
    if (typeof error === "string") return new IpcError(error);
    if (error instanceof Error) return new IpcError(error.message);
    if (error && typeof error === "object" && "message" in error) {
      const e = error as Partial<DbError> & Pick<ConfigError, "message">;
      return new IpcError(e.message, {
        kind: typeof e.kind === "string" ? e.kind : undefined,
        code: e.code,
        hint: e.hint,
        detail: e.detail,
      });
    }
    return new IpcError(String(error));
  }
}

export function isPasswordRequired(error: unknown): boolean {
  return error instanceof IpcError && error.code === PASSWORD_REQUIRED;
}

export function isAuthError(error: unknown): boolean {
  return error instanceof IpcError && error.kind === "auth";
}

/** The message to show for any thrown value. */
export function errorMessage(error: unknown): string {
  return IpcError.from(error).message;
}
