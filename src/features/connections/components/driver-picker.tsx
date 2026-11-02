import { CircleAlertIcon } from "lucide-react";

import { Alert, AlertTitle } from "@/components/ui/alert";
import { RadioGroup } from "@/components/ui/radio-group";
import { Spinner } from "@/components/ui/spinner";
import { useDriversStore } from "@/stores/drivers";

import { DriverTile } from "./driver-tile";

/**
 * Every driver the backend offers, as a radio grid (arrow keys move the
 * choice). Double-click or Enter continues.
 */
export function DriverPicker({
  value,
  onChange,
  onContinue,
  labelledBy,
}: {
  value: string | null;
  onChange: (driverId: string) => void;
  onContinue: () => void;
  labelledBy: string;
}) {
  const { drivers, status, error } = useDriversStore();

  if (status === "error") {
    return (
      <Alert variant="destructive">
        <CircleAlertIcon />
        <AlertTitle>{error}</AlertTitle>
      </Alert>
    );
  }
  if (status !== "ready") {
    return (
      <div className="flex h-16 items-center justify-center text-muted-foreground">
        <Spinner />
      </div>
    );
  }

  return (
    <RadioGroup
      aria-labelledby={labelledBy}
      value={value}
      onValueChange={(v) => onChange(v as string)}
      onKeyDown={(e) => {
        if (e.key === "Enter" && value) {
          e.preventDefault();
          onContinue();
        }
      }}
      className="grid-cols-2 sm:grid-cols-3"
    >
      {drivers.map((d) => (
        <DriverTile key={d.id} driver={d} onActivate={onContinue} />
      ))}
    </RadioGroup>
  );
}
