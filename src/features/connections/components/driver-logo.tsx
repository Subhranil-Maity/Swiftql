import { DatabaseIcon } from "lucide-react";
import { cn } from "@/lib/utils";

import { useDriver } from "@/stores/drivers";

/** A driver's logo, or a generic database icon if it has none. Decorative. */
export function DriverLogo({ driverId, className }: { driverId: string; className?: string }) {
  const driver = useDriver(driverId);
  if (!driver?.image) {
    return <DatabaseIcon aria-hidden className={cn("size-4 text-muted-foreground", className)} />;
  }
  return <img src={driver.image} alt="" aria-hidden className={cn("size-4 shrink-0", className)} />;
}
