import { useId } from "react";

import { Checkbox } from "@/components/ui/checkbox";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { AuthKind, DriverInfo } from "@/lib/ipc";

import type { ConnectionFormApi } from "../hooks/use-connection-form";
import { AUTH_LABELS, authKinds, needsUser } from "../model/form-state";

/** Who you connect as and how you prove it. */
export function AuthFields({ api, driver }: { api: ConnectionFormApi; driver: DriverInfo }) {
  const { form, errors, set } = api;
  const id = useId();
  const items = authKinds(driver).map((k) => ({ value: k, label: AUTH_LABELS[k] }));

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <Field>
          <FieldLabel htmlFor={`${id}-kind`}>Authentication</FieldLabel>
          <Select
            items={items}
            value={form.authKind}
            onValueChange={(v) => v && set("authKind", v as AuthKind)}
          >
            <SelectTrigger id={`${id}-kind`} className="w-full">
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
        </Field>

        {needsUser(form.authKind) && (
          <Field data-invalid={!!errors.user}>
            <FieldLabel htmlFor={`${id}-user`}>User</FieldLabel>
            <Input
              id={`${id}-user`}
              value={form.user}
              onChange={(e) => set("user", e.currentTarget.value)}
              aria-invalid={!!errors.user}
              spellCheck={false}
              autoComplete="username"
            />
            <FieldError>{errors.user}</FieldError>
          </Field>
        )}
      </div>

      {form.authKind === "password" && (
        <div className="grid items-end gap-4 sm:grid-cols-2">
          <Field>
            <FieldLabel htmlFor={`${id}-password`}>Password</FieldLabel>
            <Input
              id={`${id}-password`}
              type="password"
              value={form.password}
              onChange={(e) => set("password", e.currentTarget.value)}
              placeholder={form.passwordSaved ? "Saved; type to replace" : undefined}
              autoComplete="current-password"
            />
          </Field>
          <Field orientation="horizontal" className="h-8">
            <Checkbox
              id={`${id}-save`}
              checked={form.savePassword}
              onCheckedChange={(checked) => set("savePassword", checked)}
            />
            <FieldLabel htmlFor={`${id}-save`} className="font-normal">
              Save password in the system keychain
            </FieldLabel>
          </Field>
        </div>
      )}

      {form.authKind === "passwordPrompt" && (
        <FieldDescription>You'll be asked for the password each time you connect.</FieldDescription>
      )}
      {form.authKind === "pgPass" && (
        <FieldDescription>
          The password is read from your .pgpass file or the PGPASSWORD variable.
        </FieldDescription>
      )}
    </div>
  );
}
