import { Field, FieldLabel, FieldTitle } from "@/components/ui/field";
import { RadioGroupItem } from "@/components/ui/radio-group";
import type { DriverInfo } from "@/lib/ipc";

import { DriverLogo } from "./driver-logo";

/**
 * One choice in the driver picker: a radio styled as a tile. The padding
 * overrides FieldLabel's `*:data-[slot=field]:p-2.5` through the same child
 * selector (a plain class on the Field would lose on specificity), and the
 * name truncates so the radio never gets pushed into the border.
 */
export function DriverTile({ driver, onActivate }: { driver: DriverInfo; onActivate: () => void }) {
  const id = `driver-${driver.id}`;
  return (
    <FieldLabel htmlFor={id} onDoubleClick={onActivate} className="min-w-0 *:data-[slot=field]:px-3.5 *:data-[slot=field]:py-3">
      <Field orientation="horizontal" className="min-w-0 items-center gap-3">
        <DriverLogo driverId={driver.id} className="size-6 shrink-0" />
        <FieldTitle className="block min-w-0 flex-1 truncate">{driver.name}</FieldTitle>
        <RadioGroupItem id={id} value={driver.id} className="shrink-0" />
      </Field>
    </FieldLabel>
  );
}
