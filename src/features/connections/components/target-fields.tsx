import { useId } from "react";

import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { defaultPort } from "@/lib/connection-format";
import type { DriverInfo, TargetKind } from "@/lib/ipc";

import type { ConnectionFormApi } from "../hooks/use-connection-form";
import { FilePathInput } from "./file-path-input";
import { TARGET_LABELS, targetKinds } from "../model/form-state";

/** Where the database is: host/port/database, a URL, a socket or a file. */
export function TargetFields({ api, driver }: { api: ConnectionFormApi; driver: DriverInfo }) {
  const { form, errors, set } = api;
  const id = useId();
  const kinds = targetKinds(driver);
  const port = defaultPort(driver.id);

  return (
    <div className="flex flex-col gap-4">
      {kinds.length > 1 && (
        <Field>
          <FieldLabel id={`${id}-method`}>Connect with</FieldLabel>
          <ToggleGroup
            aria-labelledby={`${id}-method`}
            value={[form.method]}
            onValueChange={(next) => next[0] && set("method", next[0] as TargetKind)}
            variant="outline"
            size="sm"
            spacing={0}
          >
            {kinds.map((k) => (
              <ToggleGroupItem key={k} value={k}>
                {TARGET_LABELS[k]}
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
        </Field>
      )}

      {form.method === "host" && (
        <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_6rem_minmax(0,1fr)]">
          <Field data-invalid={!!errors.host}>
            <FieldLabel htmlFor={`${id}-host`}>Host</FieldLabel>
            <Input
              id={`${id}-host`}
              value={form.host}
              onChange={(e) => set("host", e.currentTarget.value)}
              placeholder="localhost"
              aria-invalid={!!errors.host}
              spellCheck={false}
              autoComplete="off"
              className="font-mono"
            />
            <FieldError>{errors.host}</FieldError>
          </Field>
          <Field data-invalid={!!errors.port}>
            <FieldLabel htmlFor={`${id}-port`}>Port</FieldLabel>
            <Input
              id={`${id}-port`}
              value={form.port}
              onChange={(e) => set("port", e.currentTarget.value)}
              placeholder={port ? String(port) : undefined}
              inputMode="numeric"
              aria-invalid={!!errors.port}
              className="font-mono"
            />
            <FieldError>{errors.port}</FieldError>
          </Field>
          <DatabaseField id={`${id}-db`} api={api} />
        </div>
      )}

      {form.method === "url" && (
        <Field data-invalid={!!errors.url}>
          <FieldLabel htmlFor={`${id}-url`}>URL</FieldLabel>
          <Input
            id={`${id}-url`}
            value={form.url}
            onChange={(e) => set("url", e.currentTarget.value)}
            placeholder={`${driver.id}://db.example.com:${port ?? ""}/app`}
            aria-invalid={!!errors.url}
            spellCheck={false}
            autoComplete="off"
            className="font-mono"
          />
          <FieldDescription>
            Only the host, port and database are read from the URL. Set the user and
            password under Authentication.
          </FieldDescription>
          <FieldError>{errors.url}</FieldError>
        </Field>
      )}

      {form.method === "socket" && (
        <div className="grid gap-4 sm:grid-cols-2">
          <Field data-invalid={!!errors.socketPath}>
            <FieldLabel htmlFor={`${id}-socket`}>Socket directory</FieldLabel>
            <Input
              id={`${id}-socket`}
              value={form.socketPath}
              onChange={(e) => set("socketPath", e.currentTarget.value)}
              placeholder="/var/run/postgresql"
              aria-invalid={!!errors.socketPath}
              spellCheck={false}
              className="font-mono"
            />
            <FieldError>{errors.socketPath}</FieldError>
          </Field>
          <DatabaseField id={`${id}-db`} api={api} />
        </div>
      )}

      {form.method === "file" && (
        <Field data-invalid={!!errors.filePath}>
          <FieldLabel htmlFor={`${id}-file`}>Database file</FieldLabel>
          <FilePathInput
            id={`${id}-file`}
            value={form.filePath}
            onChange={(v) => set("filePath", v)}
            invalid={!!errors.filePath}
          />
          <FieldError>{errors.filePath}</FieldError>
        </Field>
      )}
    </div>
  );
}

function DatabaseField({ id, api }: { id: string; api: ConnectionFormApi }) {
  const error = api.errors.database;
  return (
    <Field data-invalid={!!error}>
      <FieldLabel htmlFor={id}>Database</FieldLabel>
      <Input
        id={id}
        value={api.form.database}
        onChange={(e) => api.set("database", e.currentTarget.value)}
        aria-invalid={!!error}
        spellCheck={false}
        autoComplete="off"
        className="font-mono"
      />
      <FieldError>{error}</FieldError>
    </Field>
  );
}
