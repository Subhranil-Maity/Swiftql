import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { api, type SessionInfo } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { cn } from "@/lib/utils";
import { useQueryRunsStore } from "@/stores/query-runs";
import { useSessionsStore } from "@/stores/sessions";

const LABELS = {
  inTransaction: { text: "In transaction", hint: "Changes aren't visible to others until you commit." },
  failed: {
    text: "Transaction failed",
    hint: "A statement failed; every statement now errors until you roll back.",
  },
} as const;

/** While a transaction is open: a badge, and Commit / Rollback. Nothing when idle. */
export function TransactionStatus({ session }: { session: SessionInfo }) {
  const setTxStatus = useSessionsStore((s) => s.setTxStatus);
  const running = useQueryRunsStore((s) => s.runningTab[session.sessionId] !== undefined);
  const [ending, setEnding] = useState<"commit" | "rollback" | null>(null);
  const status = session.txStatus;
  if (status === "idle") return null;
  const label = LABELS[status];

  const end = async (commit: boolean) => {
    setEnding(commit ? "commit" : "rollback");
    try {
      setTxStatus(session.sessionId, await api.endTransaction(session.sessionId, commit));
    } catch (error) {
      toast.error(commit ? "Couldn't commit" : "Couldn't roll back", { description: errorMessage(error) });
    } finally {
      setEnding(null);
    }
  };

  return (
    <div className="flex items-center gap-1.5">
      <Tooltip>
        <TooltipTrigger
          render={<span />}
          className={cn(
            "flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-xs",
            status === "failed" ? "border-destructive/40 text-destructive" : "border-warning/50 text-warning",
          )}
        >
          <span
            aria-hidden
            className={cn("size-1.5 rounded-full", status === "failed" ? "bg-destructive" : "bg-warning")}
          />
          {label.text}
        </TooltipTrigger>
        <TooltipContent>{label.hint}</TooltipContent>
      </Tooltip>
      {status === "inTransaction" && (
        <Button size="xs" variant="outline" disabled={running || ending !== null} onClick={() => void end(true)}>
          Commit
        </Button>
      )}
      <Button size="xs" variant="ghost" disabled={running || ending !== null} onClick={() => void end(false)}>
        Rollback
      </Button>
    </div>
  );
}
