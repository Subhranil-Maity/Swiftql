import { useId } from "react";

import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { FieldSpec } from "@/lib/ipc";

/** Renders one driver-specific option (`DriverInfo.extraOptions`) by its kind. */
export function ExtraOptionField({
  spec,
  value,
  onChange,
}: {
  spec: FieldSpec;
  value: string;
  onChange: (value: string) => void;
}) {
  const id = useId();
  const label = spec.required ? spec.label : `${spec.label} (optional)`;

  switch (spec.kind.type) {
    case "bool":
      return (
        <Field orientation="horizontal">
          <Checkbox
            id={id}
            checked={value === "true"}
            onCheckedChange={(checked) => onChange(checked ? "true" : "false")}
          />
          <FieldLabel htmlFor={id} className="font-normal">
            {spec.label}
          </FieldLabel>
        </Field>
      );
    case "select": {
      const items = spec.kind.options.map((o) => ({ value: o, label: o }));
      return (
        <Field>
          <FieldLabel htmlFor={id}>{label}</FieldLabel>
          <Select items={items} value={value || null} onValueChange={(v) => onChange(v ?? "")}>
            <SelectTrigger id={id} className="w-full">
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
      );
    }
    default:
      // TODO: kind "file" should open a native file picker
      return (
        <Field>
          <FieldLabel htmlFor={id}>{label}</FieldLabel>
          <Input
            id={id}
            type={spec.kind.type === "password" ? "password" : "text"}
            inputMode={spec.kind.type === "number" ? "numeric" : undefined}
            value={value}
            onChange={(e) => onChange(e.currentTarget.value)}
            placeholder={spec.default ?? undefined}
            spellCheck={false}
          />
        </Field>
      );
  }
}
