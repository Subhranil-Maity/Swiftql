import { useId } from "react";

import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";

import type { ConnectionFormApi } from "../hooks/use-connection-form";
import { ColorSwatches } from "./color-swatches";

/** Name and colour: how the connection shows up in the list. */
export function GeneralFields({ form, errors, set }: ConnectionFormApi) {
  const id = useId();
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <Field data-invalid={!!errors.name}>
        <FieldLabel htmlFor={`${id}-name`}>Name</FieldLabel>
        <Input
          id={`${id}-name`}
          value={form.name}
          onChange={(e) => set("name", e.currentTarget.value)}
          placeholder="Orders prod"
          aria-invalid={!!errors.name}
          autoFocus
          autoComplete="off"
        />
        <FieldError>{errors.name}</FieldError>
      </Field>
      <Field>
        <FieldLabel id={`${id}-color`}>Colour</FieldLabel>
        <ColorSwatches
          value={form.color}
          onChange={(c) => set("color", c)}
          labelledBy={`${id}-color`}
        />
      </Field>
    </div>
  );
}
