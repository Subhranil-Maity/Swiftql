import { useLayoutEffect, useRef } from "react";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { MessageEntry, RunError } from "@/features/console/model/run";
import type { Notice } from "@/lib/ipc";
import { cn } from "@/lib/utils";

const time = (at: number) => new Date(at).toLocaleTimeString([], { hour12: false });

/** "0.4 ms", "42 ms", "1.25 s". */
export function formatDuration(ms: number): string {
  if (ms < 1) return `${ms.toFixed(1)} ms`;
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

const SEVERITY_LABEL: Record<Notice["severity"], string> = {
  warning: "WARNING",
  notice: "NOTICE",
  info: "INFO",
  log: "LOG",
  debug: "DEBUG",
};

function Extra({ label, text }: { label: string; text: string | null }) {
  if (!text) return null;
  return (
    <p className="text-muted-foreground">
      <span className="text-foreground/70">{label}:</span> {text}
    </p>
  );
}

function ErrorLine({
  error,
  elapsedMs,
  onJump,
}: {
  error: RunError;
  elapsedMs?: number;
  onJump: (offset: number) => void;
}) {
  if (error.kind === "cancelled") {
    return (
      <p className="text-muted-foreground">
        Cancelled{elapsedMs != null && ` · ${formatDuration(elapsedMs)}`}
      </p>
    );
  }
  return (
    <div className="space-y-0.5 border-l-2 border-destructive py-0.5 pl-2.5">
      <p className="text-destructive">
        <span className="font-medium">ERROR</span>
        {error.code && <span className="opacity-75"> {error.code}</span>}
        {": "}
        {error.message}
      </p>
      <Extra label="Detail" text={error.detail} />
      <Extra label="Hint" text={error.hint} />
      {error.location && (
        <Button
          variant="link"
          size="xs"
          className="h-auto p-0 font-mono text-xs text-destructive"
          onClick={() => error.location && onJump(error.location.offset)}
        >
          Line {error.location.line}, col {error.location.column}
        </Button>
      )}
    </div>
  );
}

function Entry({ entry, onJump }: { entry: MessageEntry; onJump: (offset: number) => void }) {
  switch (entry.kind) {
    case "run-start":
      return (
        <p className="pt-3 font-sans text-xs text-muted-foreground first:pt-0">
          {entry.runKind === "script" ? "Run script" : "Run statement"} · {time(entry.at)}
        </p>
      );
    case "statement-done":
      return (
        <p className="text-muted-foreground">
          <span className="text-foreground/80">{entry.tag}</span> · {formatDuration(entry.elapsedMs)}
          {entry.resultState === "truncated" && " · stopped at the memory limit (Settings → Query & results)"}
          {entry.resultState === "cancelled" && " · stopped; rows read so far are kept"}
        </p>
      );
    case "notice": {
      const { notice } = entry;
      const warning = notice.severity === "warning";
      return (
        <div
          className={cn(
            "space-y-0.5 border-l-2 py-0.5 pl-2.5",
            warning ? "border-warning" : "border-(--conn-blue)",
          )}
        >
          <p>
            <span className={cn("font-medium", warning ? "text-warning" : "text-(--conn-blue)")}>
              {SEVERITY_LABEL[notice.severity]}
            </span>
            {": "}
            {notice.message}
          </p>
          <Extra label="Detail" text={notice.detail} />
          <Extra label="Hint" text={notice.hint} />
        </div>
      );
    }
    case "error":
      return <ErrorLine error={entry.error} elapsedMs={entry.elapsedMs} onJump={onJump} />;
    case "skipped":
      return (
        <p className="text-muted-foreground">
          {entry.count === 1 ? "1 statement skipped" : `${entry.count} statements skipped`}
        </p>
      );
  }
}

/** Everything the server said in a tab's runs, newest at the bottom. */
export function MessagesLog({
  entries,
  onJump,
}: {
  entries: MessageEntry[];
  /** Moves the editor cursor to an offset, e.g. an error's position. */
  onJump: (offset: number) => void;
}) {
  const contentRef = useRef<HTMLDivElement>(null);
  const atBottom = useRef(true);

  // Follow new messages unless the user has scrolled up to read older ones.
  useLayoutEffect(() => {
    const viewport = contentRef.current?.closest<HTMLElement>("[data-slot=scroll-area-viewport]");
    if (!viewport) return;
    if (atBottom.current) viewport.scrollTop = viewport.scrollHeight;
    const onScroll = () => {
      atBottom.current = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight < 24;
    };
    viewport.addEventListener("scroll", onScroll);
    return () => viewport.removeEventListener("scroll", onScroll);
  }, [entries]);

  if (entries.length === 0) {
    return <p className="p-4 text-muted-foreground">No messages.</p>;
  }

  return (
    <ScrollArea className="h-full min-h-0">
      <div
        ref={contentRef}
        role="log"
        aria-label="Messages"
        className="space-y-1.5 px-4 py-3 font-mono text-xs leading-relaxed break-words whitespace-pre-wrap"
      >
        {entries.map((entry) => (
          <Entry key={entry.id} entry={entry} onJump={onJump} />
        ))}
      </div>
    </ScrollArea>
  );
}
