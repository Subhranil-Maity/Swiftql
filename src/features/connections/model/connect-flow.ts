// What a failed connect or test asks the user for before trying again: a
// database password, an SSH password or key passphrase, or a look at an SSH
// host key that isn't known yet.
import { IpcError } from "@/lib/ipc-error";

/** Codes from `driver_core::connect::ssh_codes` and `connect.rs`. */
export const CODES = {
  password: "PASSWORD_REQUIRED",
  sshPassword: "SSH_PASSWORD_REQUIRED",
  sshPassphrase: "SSH_PASSPHRASE_REQUIRED",
  hostUnknown: "SSH_HOST_UNKNOWN",
  hostChanged: "SSH_HOST_CHANGED",
} as const;

export type SecretKind = "password" | "sshPassword" | "sshPassphrase";

/** Typed once for this attempt, never saved. */
export type Secrets = { password?: string; sshSecret?: string };

export type HostKeyInfo = {
  host: string;
  port: number;
  keyType: string;
  fingerprint: string;
  /** OpenSSH public key line, for `trustSshHost`. */
  key: string;
};

export type NextStep = { kind: "secret"; secret: SecretKind } | { kind: "hostKey"; info: HostKeyInfo } | null;

/** What to ask for after `error`, or null when it's a plain failure. */
export function nextStep(error: unknown): NextStep {
  const e = IpcError.from(error);
  switch (e.code) {
    case CODES.password:
      return { kind: "secret", secret: "password" };
    case CODES.sshPassword:
      return { kind: "secret", secret: "sshPassword" };
    case CODES.sshPassphrase:
      return { kind: "secret", secret: "sshPassphrase" };
    case CODES.hostUnknown: {
      try {
        const info = JSON.parse(e.detail ?? "") as HostKeyInfo;
        if (info && typeof info.key === "string") return { kind: "hostKey", info };
      } catch {
        // fall through: show it as an error
      }
      return null;
    }
    default:
      return null;
  }
}

/** `secrets` with the answer to a `kind` prompt. */
export function withSecret(secrets: Secrets, kind: SecretKind, value: string): Secrets {
  return kind === "password" ? { ...secrets, password: value } : { ...secrets, sshSecret: value };
}

export const SECRET_LABELS: Record<SecretKind, { title: string; label: string; autoComplete: string }> = {
  password: { title: "Database password", label: "Password", autoComplete: "current-password" },
  sshPassword: { title: "SSH password", label: "SSH password", autoComplete: "current-password" },
  sshPassphrase: { title: "SSH key passphrase", label: "Passphrase", autoComplete: "off" },
};
