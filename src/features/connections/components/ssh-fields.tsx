import { useEffect, useId, useState } from "react";

import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldDescription, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { api as ipc, type DriverInfo, type SshConfigHost } from "@/lib/ipc";

import type { ConnectionFormApi } from "../hooks/use-connection-form";
import { SSH_AUTH_LABELS, type SshAuthKind, type SshSourceKind } from "../model/form-state";
import { FilePathInput } from "./file-path-input";

const AUTH_ORDER: SshAuthKind[] = ["auto", "key", "password", "agent"];

/** Hosts from ~/.ssh/config, read once per form. */
function useSshHosts(enabled: boolean) {
  const [hosts, setHosts] = useState<SshConfigHost[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!enabled || hosts) return;
    ipc
      .listSshHosts()
      .then(setHosts)
      .catch((e: unknown) => {
        setHosts([]);
        setError(e instanceof Error ? e.message : String(e));
      });
  }, [enabled, hosts]);
  return { hosts: hosts ?? [], loading: enabled && hosts === null, error };
}

/** "ops@bastion.example.com:2222 via jump1" */
function describeHost(h: SshConfigHost): string {
  const user = h.user ? `${h.user}@` : "";
  const port = h.port === 22 ? "" : `:${h.port}`;
  const via = h.proxyJump.length ? ` via ${h.proxyJump.join(" → ")}` : "";
  return `${user}${h.hostName}${port}${via}`;
}

/** An SSH tunnel to reach the database through a jump host. */
export function SshFields({ api, driver }: { api: ConnectionFormApi; driver: DriverInfo }) {
  const { form, errors, set } = api;
  const id = useId();
  const { hosts, loading, error } = useSshHosts(form.sshEnabled && form.sshSource === "config");
  if (!driver.capabilities.sshTunnel) return null;

  const hostItems = hosts.map((h) => ({ value: h.alias, label: h.alias }));
  const selected = hosts.find((h) => h.alias === form.sshAlias);
  const authItems = AUTH_ORDER.map((k) => ({ value: k, label: SSH_AUTH_LABELS[k] }));
  const hasSecret = form.sshAuth === "password" || form.sshAuth === "key";

  return (
    <div className="flex flex-col gap-4 rounded-lg border p-4">
      <Field orientation="horizontal" data-invalid={!!errors.sshEnabled}>
        <Switch
          id={`${id}-enabled`}
          checked={form.sshEnabled}
          onCheckedChange={(checked) => set("sshEnabled", checked)}
        />
        <div className="flex flex-col gap-0.5">
          <FieldLabel htmlFor={`${id}-enabled`}>SSH tunnel</FieldLabel>
          <FieldDescription>Reach the database through an SSH server, as it sees the host above.</FieldDescription>
          <FieldError>{errors.sshEnabled}</FieldError>
        </div>
      </Field>

      {form.sshEnabled && (
        <>
          <Field>
            <FieldLabel id={`${id}-source`}>Server</FieldLabel>
            <ToggleGroup
              aria-labelledby={`${id}-source`}
              value={[form.sshSource]}
              onValueChange={(next) => next[0] && set("sshSource", next[0] as SshSourceKind)}
              variant="outline"
              size="sm"
              spacing={0}
            >
              <ToggleGroupItem value="manual">Enter details</ToggleGroupItem>
              <ToggleGroupItem value="config">From ~/.ssh/config</ToggleGroupItem>
            </ToggleGroup>
          </Field>

          {form.sshSource === "config" ? (
            <Field data-invalid={!!errors.sshAlias}>
              <FieldLabel htmlFor={`${id}-alias`}>Host</FieldLabel>
              <Select
                items={hostItems}
                value={form.sshAlias || null}
                onValueChange={(v) => v && set("sshAlias", v as string)}
              >
                <SelectTrigger id={`${id}-alias`} className="w-full" aria-invalid={!!errors.sshAlias}>
                  <SelectValue placeholder={loading ? "Reading ~/.ssh/config…" : "Choose a host"} />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {hosts.map((h) => (
                      <SelectItem key={h.alias} value={h.alias}>
                        <span>{h.alias}</span>
                        <span className="ml-2 font-mono text-xs text-muted-foreground">{describeHost(h)}</span>
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
              <FieldDescription>
                {error
                  ? `Couldn't read ~/.ssh/config: ${error}`
                  : !loading && hosts.length === 0
                    ? "No Host entries found in ~/.ssh/config."
                    : selected
                      ? `${describeHost(selected)}. HostName, User, Port, IdentityFile and ProxyJump are read when connecting.`
                      : "HostName, User, Port, IdentityFile and ProxyJump are read when connecting."}
              </FieldDescription>
              <FieldError>{errors.sshAlias}</FieldError>
            </Field>
          ) : (
            <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_6rem_minmax(0,1fr)]">
              <Field data-invalid={!!errors.sshHost}>
                <FieldLabel htmlFor={`${id}-host`}>SSH host</FieldLabel>
                <Input
                  id={`${id}-host`}
                  value={form.sshHost}
                  onChange={(e) => set("sshHost", e.currentTarget.value)}
                  placeholder="bastion.example.com"
                  aria-invalid={!!errors.sshHost}
                  spellCheck={false}
                />
                <FieldError>{errors.sshHost}</FieldError>
              </Field>
              <Field data-invalid={!!errors.sshPort}>
                <FieldLabel htmlFor={`${id}-port`}>Port</FieldLabel>
                <Input
                  id={`${id}-port`}
                  value={form.sshPort}
                  onChange={(e) => set("sshPort", e.currentTarget.value)}
                  placeholder="22"
                  inputMode="numeric"
                  aria-invalid={!!errors.sshPort}
                />
                <FieldError>{errors.sshPort}</FieldError>
              </Field>
              <Field data-invalid={!!errors.sshUser}>
                <FieldLabel htmlFor={`${id}-user`}>SSH user</FieldLabel>
                <Input
                  id={`${id}-user`}
                  value={form.sshUser}
                  onChange={(e) => set("sshUser", e.currentTarget.value)}
                  aria-invalid={!!errors.sshUser}
                  spellCheck={false}
                  autoComplete="off"
                />
                <FieldError>{errors.sshUser}</FieldError>
              </Field>
            </div>
          )}

          <div className="grid gap-4 sm:grid-cols-2">
            <Field>
              <FieldLabel htmlFor={`${id}-auth`}>SSH authentication</FieldLabel>
              <Select
                items={authItems}
                value={form.sshAuth}
                onValueChange={(v) => v && set("sshAuth", v as SshAuthKind)}
              >
                <SelectTrigger id={`${id}-auth`} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {authItems.map((item) => (
                      <SelectItem key={item.value} value={item.value}>
                        {item.label}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
              {form.sshAuth === "auto" && (
                <FieldDescription>
                  Tries IdentityFile keys from ~/.ssh/config (or id_ed25519, id_ecdsa, id_rsa), then the SSH agent.
                </FieldDescription>
              )}
              {form.sshAuth === "agent" && (
                <FieldDescription>Uses the OpenSSH agent or Pageant on Windows, SSH_AUTH_SOCK elsewhere.</FieldDescription>
              )}
            </Field>

            {form.sshAuth === "password" && (
              <Field>
                <FieldLabel htmlFor={`${id}-password`}>SSH password</FieldLabel>
                <Input
                  id={`${id}-password`}
                  type="password"
                  value={form.sshPassword}
                  onChange={(e) => set("sshPassword", e.currentTarget.value)}
                  placeholder={form.sshSecretSaved ? "Saved; type to replace" : undefined}
                  autoComplete="off"
                />
              </Field>
            )}
            {form.sshAuth === "key" && (
              <Field data-invalid={!!errors.sshKeyPath}>
                <FieldLabel htmlFor={`${id}-key`}>Private key</FieldLabel>
                <FilePathInput
                  id={`${id}-key`}
                  value={form.sshKeyPath}
                  onChange={(v) => set("sshKeyPath", v)}
                  placeholder="~/.ssh/id_ed25519"
                  invalid={!!errors.sshKeyPath}
                />
                <FieldError>{errors.sshKeyPath}</FieldError>
              </Field>
            )}
          </div>

          {form.sshAuth === "key" && (
            <Field className="sm:w-1/2 sm:pr-2">
              <FieldLabel htmlFor={`${id}-passphrase`}>Key passphrase</FieldLabel>
              <Input
                id={`${id}-passphrase`}
                type="password"
                value={form.sshPassphrase}
                onChange={(e) => set("sshPassphrase", e.currentTarget.value)}
                placeholder={form.sshSecretSaved ? "Saved; type to replace" : "None, or ask when needed"}
                autoComplete="off"
              />
            </Field>
          )}

          {hasSecret && (
            <Field orientation="horizontal">
              <Checkbox
                id={`${id}-save`}
                checked={form.sshSaveSecret}
                onCheckedChange={(checked) => set("sshSaveSecret", checked)}
              />
              <FieldLabel htmlFor={`${id}-save`} className="font-normal">
                Save the SSH {form.sshAuth === "password" ? "password" : "passphrase"} in the system keychain
              </FieldLabel>
            </Field>
          )}
        </>
      )}
    </div>
  );
}
