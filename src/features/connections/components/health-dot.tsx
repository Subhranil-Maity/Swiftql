import { Spinner } from "@/components/ui/spinner";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { formatVersion } from "@/lib/connection-format";
import type { HealthState } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { HealthEntry } from "@/stores/health";

const DOT: Record<HealthState, string> = {
  online: "bg-success",
  misconfigured: "bg-warning",
  offline: "bg-destructive",
  needsPassword: "bg-muted-foreground/60",
};

function describe(entry: Exclude<HealthEntry, "checking">): string {
  switch (entry.state) {
    case "online": {
      const v = entry.serverVersion;
      return v ? `Online · ${v.product} ${formatVersion(v)}` : "Online";
    }
    case "misconfigured":
      return `Reachable, but: ${entry.message ?? "the server refused the connection"}`;
    case "offline":
      return `Offline: ${entry.message ?? "the server can't be reached"}`;
    case "needsPassword":
      return "Needs a password to check";
  }
}

/**
 * Result of the live health check: green online, orange reachable but
 * misconfigured, red offline, grey not checked. Sits inside the row's
 * button, so the tooltip trigger is a span, not another button.
 */
export function HealthDot({ entry, name }: { entry: HealthEntry; name: string }) {
  if (entry === "checking") {
    return (
      <span className="flex text-muted-foreground">
        <Spinner aria-hidden className="size-3" />
        <span className="sr-only">Checking {name}</span>
      </span>
    );
  }

  const text = describe(entry);
  return (
    <Tooltip>
      <TooltipTrigger render={<span />} className="flex size-4 items-center justify-center">
        <span aria-hidden className={cn("size-2 rounded-full", DOT[entry.state])} />
        <span className="sr-only">{text}</span>
      </TooltipTrigger>
      <TooltipContent side="left" className="max-w-sm break-words">
        {text}
      </TooltipContent>
    </Tooltip>
  );
}
