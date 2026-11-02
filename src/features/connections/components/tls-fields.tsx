import { useId } from "react";

import { Field, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

import { SSL_LABELS } from "../model/form-state";

const ITEMS = [{ value: "disable", label: SSL_LABELS.disable }];
const NOTE = "SSL isn't supported yet. Connections are unencrypted.";

/**
 * SSL mode. Locked to "Disable" until the driver can do TLS; the tooltip
 * says why.
 */
// TODO: re-enable SSL modes (and the CA certificate field) once
// driver-postgres supports TLS (tls.rs).
export function TlsFields() {
  const id = useId();

  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <Field data-disabled>
        <FieldLabel htmlFor={`${id}-ssl`}>SSL</FieldLabel>
        <Tooltip>
          {/* Disabled controls get no hover or focus, so a focusable span carries the tooltip. */}
          <TooltipTrigger
            render={<span tabIndex={0} aria-describedby={`${id}-ssl-note`} />}
            className="block rounded-md outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
          >
            <Select items={ITEMS} value="disable" disabled>
              <SelectTrigger id={`${id}-ssl`} className="pointer-events-none w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {ITEMS.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {item.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </TooltipTrigger>
          <TooltipContent>{NOTE}</TooltipContent>
        </Tooltip>
        <span id={`${id}-ssl-note`} className="sr-only">
          {NOTE}
        </span>
      </Field>
    </div>
  );
}
