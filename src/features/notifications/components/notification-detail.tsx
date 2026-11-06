import { CopyIcon } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";

import type { NotificationEntry } from "../stores/notifications";

/** The payload pretty-printed when it's a JSON object or array. */
function pretty(payload: string): { text: string; json: boolean } {
  try {
    const value: unknown = JSON.parse(payload);
    if (value !== null && typeof value === "object") return { text: JSON.stringify(value, null, 2), json: true };
  } catch {
    // plain text
  }
  return { text: payload, json: false };
}

/** One notification in full. */
export function NotificationDetail({ entry }: { entry: NotificationEntry }) {
  const { text, json } = pretty(entry.payload);
  return (
    <div className="flex max-h-64 shrink-0 flex-col border-t">
      <div className="flex items-center gap-2 px-3 py-1.5 text-xs text-muted-foreground">
        <span className="font-mono text-foreground">{entry.channel}</span>
        <span>· pid {entry.processId}</span>
        <span>· {entry.source === "console" ? "console session" : "listener"}</span>
        {json && <span>· JSON</span>}
        <Button
          size="icon-xs"
          variant="ghost"
          className="ml-auto"
          aria-label="Copy payload"
          onClick={() =>
            void navigator.clipboard.writeText(entry.payload).then(
              () => toast.success("Payload copied"),
              () => toast.error("Couldn't copy to the clipboard"),
            )
          }
        >
          <CopyIcon />
        </Button>
      </div>
      <pre className="min-h-0 flex-1 overflow-auto px-3 pb-3 font-mono text-xs break-all whitespace-pre-wrap select-text">
        {text || "(empty payload)"}
      </pre>
    </div>
  );
}
