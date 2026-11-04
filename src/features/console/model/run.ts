// What a console run produced: one result per statement sent, plus a log of
// everything the server said.
import type { ColumnMeta, DbValue, Notice, RunEvent, RunSummary } from "@/lib/ipc";

export type RunKind = "statement" | "script";

/** A failed statement or run, as the UI shows it. */
export type RunError = {
  kind: string;
  message: string;
  code: string | null;
  detail: string | null;
  hint: string | null;
  /** Where the error points in the editor, if the server gave a position. */
  location: { offset: number; line: number; column: number } | null;
};

export type StatementResult = {
  index: number;
  sql: string;
  /** Editor offset of the statement's first character when it was run. */
  offset: number;
  status: "pending" | "running" | "done" | "failed" | "skipped";
  /** Set for statements that return rows. */
  columns?: ColumnMeta[];
  rows: DbValue[][];
  tag?: string;
  elapsedMs?: number;
  truncated: boolean;
  error?: RunError;
};

type Entry<K extends string, T> = { id: number; at: number; kind: K } & T;

export type MessageEntry =
  | Entry<"run-start", { runKind: RunKind; statements: number }>
  | Entry<"statement-done", { index: number; tag: string; elapsedMs: number; truncated: boolean }>
  | Entry<"notice", { index: number; notice: Notice }>
  | Entry<"error", { index: number | null; error: RunError; elapsedMs?: number }>
  | Entry<"skipped", { count: number }>;

export type MessageInput = MessageEntry extends infer E
  ? E extends MessageEntry
    ? Omit<E, "id" | "at">
    : never
  : never;

export type RunStatus = "running" | "done" | "failed" | "cancelled";

/** "messages", or the index of a statement whose rows are shown. */
export type OutputView = "messages" | number;

export type TabRun = {
  status: RunStatus;
  kind: RunKind;
  statements: StatementResult[];
  view: OutputView;
  startedAt: number;
  summary?: RunSummary;
};

/** The statement whose rows a new run should show: the last that returned any columns. */
export function lastResultIndex(statements: StatementResult[]): number | null {
  for (let i = statements.length - 1; i >= 0; i--) {
    if (statements[i].columns) return statements[i].index;
  }
  return null;
}

/** `statement` with `event` applied. */
export function applyEvent(statement: StatementResult, event: RunEvent): StatementResult {
  switch (event.event) {
    case "statementStarted":
      return { ...statement, status: "running" };
    case "columns":
      return { ...statement, columns: event.columns };
    case "rows":
      return { ...statement, rows: statement.rows.concat(event.rows) };
    case "statementFinished":
      return {
        ...statement,
        status: "done",
        tag: event.tag,
        elapsedMs: event.elapsedMs ?? 0,
        truncated: event.truncated,
      };
    case "statementFailed":
      return { ...statement, status: "failed", elapsedMs: event.elapsedMs ?? 0 };
    case "notice":
      return statement;
  }
}
