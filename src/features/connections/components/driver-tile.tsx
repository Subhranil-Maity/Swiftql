import { Field, FieldLabel, FieldTitle } from "@/components/ui/field";
import { RadioGroupItem } from "@/components/ui/radio-group";
import type { DriverInfo } from "@/lib/ipc";

import { DriverLogo } from "./driver-logo";

/** One choice in the driver picker: a radio styled as a tile. */
export function DriverTile({ driver, onActivate }: { driver: DriverInfo; onActivate: () => void }) {
  const id = `driver-${driver.id}`;
  return (
    <FieldLabel htmlFor={id} onDoubleClick={onActivate}>
      <Field orientation="horizontal" className="items-center gap-3">
        <DriverLogo driverId={driver.id} className="size-6" />
        <FieldTitle className="flex-1">{driver.name}</FieldTitle>
        <RadioGroupItem id={id} value={driver.id} />
      </Field>
    </FieldLabel>
  );
}
