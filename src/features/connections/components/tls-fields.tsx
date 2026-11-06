import { useId } from "react";

import { Field, FieldDescription, FieldError, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { DriverInfo, SslMode } from "@/lib/ipc";

import type { ConnectionFormApi } from "../hooks/use-connection-form";
import { SSL_DESCRIPTIONS, SSL_LABELS } from "../model/form-state";
import { FilePathInput, PEM_FILTERS } from "./file-path-input";

const MODES: SslMode[] = ["disable", "prefer", "require", "verifyCa", "verifyFull"];

/** SSL mode and, for checking certificates, the CA file. */
export function TlsFields({ api, driver }: { api: ConnectionFormApi; driver: DriverInfo }) {
  const { form, errors, set } = api;
  const id = useId();
  if (!driver.capabilities.tls) return null;
  const items = MODES.map((m) => ({ value: m, label: SSL_LABELS[m] }));
  const usesCa = form.sslMode !== "disable" && form.sslMode !== "prefer";
  const socket = form.method === "socket";

  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <Field>
        <FieldLabel htmlFor={`${id}-ssl`}>SSL</FieldLabel>
        <Select items={items} value={form.sslMode} onValueChange={(v) => v && set("sslMode", v as SslMode)}>
          <SelectTrigger id={`${id}-ssl`} className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {items.map((item) => (
                <SelectItem key={item.value} value={item.value}>
                  {item.label}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
        <FieldDescription>
          {socket ? "Unix socket connections aren't encrypted." : SSL_DESCRIPTIONS[form.sslMode]}
        </FieldDescription>
      </Field>
      {usesCa && (
        <Field data-invalid={!!errors.caCert}>
          <FieldLabel htmlFor={`${id}-ca`}>CA certificate</FieldLabel>
          <FilePathInput
            id={`${id}-ca`}
            value={form.caCert}
            onChange={(v) => set("caCert", v)}
            placeholder="System certificates"
            filters={PEM_FILTERS}
            invalid={!!errors.caCert}
          />
          <FieldDescription>A PEM file, for servers with a private CA (e.g. RDS, self-signed).</FieldDescription>
          <FieldError>{errors.caCert}</FieldError>
        </Field>
      )}
    </div>
  );
}
