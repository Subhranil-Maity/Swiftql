import { lazy, Suspense, useCallback } from "react";

import { Button } from "@/components/ui/button";
import { Empty, EmptyDescription, EmptyHeader } from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { formatDuration, MessagesLog } from "@/features/console/components/messages-log";
import { ROW_LIMIT } from "@/features/console/hooks/use-run-query";
import { editorFor } from "@/features/console/model/editor-registry";
import type { MessageEntry, OutputView, TabRun } from "@/features/console/model/run";
import { cn } from "@/lib/utils";
import { useQueryRunsStore } from "@/stores/query-runs";

// The grid library is large; load it with the first result instead of at startup.
const ResultGrid = lazy(() =>
  import("@/features/console/components/result-grid").then((m) => ({ default: m.ResultGrid })),
);

const NO_MESSAGES: MessageEntry[] = [];

const TRIGGER_CLASS =
  "h-full flex-none rounded-none px-3 font-normal data-active:font-medium group-data-horizontal/tabs:after:bottom-0";

/** Red for errors, amber for notices, in the latest run. */
function messagesDot(entries: MessageEntry[]): "error" | "notice" | null {
  let dot: "error" | "notice" | null = null;
  for (let i = entries.length - 1; i >= 0; i--) {
    const entry = entries[i];
    if (entry.kind === "run-start") break;
    if (entry.kind === "error" && entry.error.kind !== "cancelled") return "error";
    if (entry.kind === "notice") dot = "notice";
  }
  return dot;
}

function rowsLabel(n: number, truncated: boolean): string {
  if (truncated) return `${n.toLocaleString()}+ rows`;
  return n === 1 ? "1 row" : `${n.toLocaleString()} rows`;
}

function RunStatus({ run }: { run: TabRun }) {
  if (run.status === "running") {
    return (
      <span className="flex items-center gap-1.5">
        <Spinner className="size-3.5" />
        Running…
      </span>
    );
  }
  const total = run.statements.length;
  const elapsed = run.summary ? ` · ${formatDuration(run.summary.elapsedMs ?? 0)}` : "";
  if (run.status === "cancelled") return <span>Cancelled{elapsed}</span>;
  if (run.status === "failed" && run.summary?.failedAt != null) {
    return (
      <span>
        Failed at statement {run.summary.failedAt + 1} of {total}
        {elapsed}
      </span>
    );
  }
  if (run.status === "failed") return <span>Failed</span>;
  return (
    <span>
      {total === 1 ? "1 statement" : `${total} statements`}
      {elapsed}
    </span>
  );
}

/** A tab's output: one tab per result set, and the Messages log. */
export function OutputPanel({ tabId }: { tabId: string }) {
  const run = useQueryRunsStore((s) => s.runs[tabId]);
  const messages = useQueryRunsStore((s) => s.messages[tabId] ?? NO_MESSAGES);
  const setView = useQueryRunsStore((s) => s.setView);
  const clearMessages = useQueryRunsStore((s) => s.clearMessages);

  const jumpTo = useCallback(
    (offset: number) => {
      const view = editorFor(tabId);
      if (!view) return;
      const anchor = Math.min(offset, view.state.doc.length);
      view.dispatch({ selection: { anchor }, scrollIntoView: true });
      view.focus();
    },
    [tabId],
  );

  if (!run && messages.length === 0) {
    return (
      <section aria-label="Output" className="flex h-full min-h-0 flex-col bg-muted/30">
        <header className="flex h-9 shrink-0 items-center px-4 text-muted-foreground">Output</header>
        <Empty className="min-h-0 p-4">
          <EmptyHeader>
            <EmptyDescription>Run a query to see its results here.</EmptyDescription>
          </EmptyHeader>
        </Empty>
      </section>
    );
  }

  const results = run?.statements.filter((st) => st.columns) ?? [];
  const view: OutputView = run?.view ?? "messages";
  const shown = typeof view === "number" ? run?.statements[view] : undefined;
  const dot = messagesDot(messages);

  return (
    <section aria-label="Output" className="flex h-full min-h-0 flex-col bg-muted/30">
      <header className="flex h-9 shrink-0 items-stretch border-b">
        <Tabs
          value={String(view)}
          onValueChange={(value) => setView(tabId, value === "messages" ? "messages" : Number(value))}
          className="min-w-0"
        >
          <TabsList
            variant="line"
            aria-label="Output views"
            className="no-scrollbar h-full w-auto min-w-0 justify-start gap-0 overflow-x-auto p-0 group-data-horizontal/tabs:h-full"
          >
            {results.map((st, i) => (
              <TabsTrigger key={st.index} value={String(st.index)} className={TRIGGER_CLASS} title={st.sql}>
                Result {i + 1}
                <span className="text-muted-foreground">· {rowsLabel(st.rows.length, st.truncated)}</span>
              </TabsTrigger>
            ))}
            <TabsTrigger value="messages" className={TRIGGER_CLASS}>
              Messages
              {dot && (
                <span
                  aria-label={dot === "error" ? "Has errors" : "Has notices"}
                  className={cn("size-1.5 rounded-full", dot === "error" ? "bg-destructive" : "bg-warning")}
                />
              )}
            </TabsTrigger>
          </TabsList>
        </Tabs>
        <div className="ml-auto flex shrink-0 items-center gap-2 px-3 text-xs text-muted-foreground">
          {view === "messages" && messages.length > 0 && (
            <Button variant="ghost" size="xs" onClick={() => clearMessages(tabId)}>
              Clear
            </Button>
          )}
          {run && <RunStatus run={run} />}
        </div>
      </header>
      <div className="min-h-0 flex-1 bg-background">
        {shown?.columns ? (
          <Suspense fallback={<Spinner className="m-4 text-muted-foreground" />}>
            <ResultGrid key={`${run?.startedAt}-${shown.index}`} result={shown} rowLimit={ROW_LIMIT} />
          </Suspense>
        ) : (
          <MessagesLog entries={messages} onJump={jumpTo} />
        )}
      </div>
    </section>
  );
}
