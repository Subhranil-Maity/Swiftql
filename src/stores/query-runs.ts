// Console runs, per tab. Kept in memory only: results are never saved.
import { create } from "zustand";

import {
  applyEvent,
  lastResultIndex,
  type MessageEntry,
  type MessageInput,
  type OutputView,
  type RunError,
  type RunKind,
  type StatementResult,
  type TabRun,
} from "@/features/console/model/run";
import type { RunEvent, RunSummary } from "@/lib/ipc";

/** Oldest messages are dropped past this. */
const MAX_MESSAGES = 2000;

let nextMessageId = 1;

interface QueryRunsState {
  /** The latest run of each tab; replaced by the next one. */
  runs: Record<string, TabRun>;
  /** Each tab's message log; kept across runs until cleared. */
  messages: Record<string, MessageEntry[]>;
  /** The tab running on each session, by session id. One run per session. */
  runningTab: Record<string, string>;

  begin: (
    tabId: string,
    sessionId: string,
    kind: RunKind,
    statements: { sql: string; offset: number }[],
  ) => void;
  /** `error` is the UI form of a `statementFailed` event's error. */
  apply: (tabId: string, event: RunEvent, error?: RunError) => void;
  finish: (tabId: string, sessionId: string, summary: RunSummary) => void;
  /** The run couldn't start or its command failed (e.g. Busy, session gone). */
  fail: (tabId: string, sessionId: string, error: RunError) => void;
  setView: (tabId: string, view: OutputView) => void;
  clearMessages: (tabId: string) => void;
  /** Forgets a closed tab. */
  discard: (tabId: string) => void;
}

function withMessages(log: MessageEntry[] | undefined, ...added: MessageInput[]): MessageEntry[] {
  const at = Date.now();
  const next = (log ?? []).concat(
    added.map((m) => ({ ...m, id: nextMessageId++, at }) as MessageEntry),
  );
  return next.length > MAX_MESSAGES ? next.slice(next.length - MAX_MESSAGES) : next;
}

function withoutKey<T>(record: Record<string, T>, key: string): Record<string, T> {
  const { [key]: _, ...rest } = record;
  return rest;
}

export const useQueryRunsStore = create<QueryRunsState>()((set) => ({
  runs: {},
  messages: {},
  runningTab: {},

  begin: (tabId, sessionId, kind, statements) =>
    set((s) => ({
      runs: {
        ...s.runs,
        [tabId]: {
          status: "running",
          kind,
          view: "messages",
          startedAt: Date.now(),
          statements: statements.map(
            (st, index): StatementResult => ({
              index,
              sql: st.sql,
              offset: st.offset,
              status: "pending",
              rows: [],
              truncated: false,
            }),
          ),
        },
      },
      messages: {
        ...s.messages,
        [tabId]: withMessages(s.messages[tabId], {
          kind: "run-start",
          runKind: kind,
          statements: statements.length,
        }),
      },
      runningTab: { ...s.runningTab, [sessionId]: tabId },
    })),

  apply: (tabId, event, error) =>
    set((s) => {
      const run = s.runs[tabId];
      const statement = run?.statements[event.index];
      if (!run || !statement) return s;

      const statements = run.statements.slice();
      statements[event.index] = { ...applyEvent(statement, event), error: error ?? statement.error };
      // Show rows as soon as a statement has columns, so long results fill in live.
      const view = event.event === "columns" ? event.index : run.view;

      let log = s.messages[tabId];
      if (event.event === "notice") {
        log = withMessages(log, { kind: "notice", index: event.index, notice: event.notice });
      } else if (event.event === "statementFinished") {
        log = withMessages(log, {
          kind: "statement-done",
          index: event.index,
          tag: event.tag,
          elapsedMs: event.elapsedMs ?? 0,
          truncated: event.truncated,
        });
      } else if (event.event === "statementFailed" && error) {
        log = withMessages(log, {
          kind: "error",
          index: event.index,
          error,
          elapsedMs: event.elapsedMs ?? 0,
        });
      }
      return {
        runs: { ...s.runs, [tabId]: { ...run, statements, view } },
        messages: log === s.messages[tabId] ? s.messages : { ...s.messages, [tabId]: log },
      };
    }),

  finish: (tabId, sessionId, summary) =>
    set((s) => {
      const run = s.runs[tabId];
      const runningTab = withoutKey(s.runningTab, sessionId);
      if (!run) return { runningTab };

      const statements = run.statements.map((st) =>
        st.status === "pending" || st.status === "running"
          ? { ...st, status: "skipped" as const }
          : st,
      );
      const skipped = statements.filter((st) => st.status === "skipped").length;
      const log =
        skipped > 0
          ? withMessages(s.messages[tabId], { kind: "skipped", count: skipped })
          : s.messages[tabId];
      const status = summary.cancelled ? "cancelled" : summary.failedAt != null ? "failed" : "done";
      return {
        runs: {
          ...s.runs,
          [tabId]: {
            ...run,
            status,
            statements,
            summary,
            view: lastResultIndex(statements) ?? "messages",
          },
        },
        messages: { ...s.messages, [tabId]: log },
        runningTab,
      };
    }),

  fail: (tabId, sessionId, error) =>
    set((s) => {
      const run = s.runs[tabId];
      return {
        runs: run
          ? {
              ...s.runs,
              [tabId]: {
                ...run,
                status: "failed",
                view: "messages",
                statements: run.statements.map((st) =>
                  st.status === "done" || st.status === "failed"
                    ? st
                    : { ...st, status: "skipped" as const },
                ),
              },
            }
          : s.runs,
        messages: {
          ...s.messages,
          [tabId]: withMessages(s.messages[tabId], { kind: "error", index: null, error }),
        },
        runningTab: withoutKey(s.runningTab, sessionId),
      };
    }),

  setView: (tabId, view) =>
    set((s) => (s.runs[tabId] ? { runs: { ...s.runs, [tabId]: { ...s.runs[tabId], view } } } : s)),

  clearMessages: (tabId) => set((s) => ({ messages: { ...s.messages, [tabId]: [] } })),

  discard: (tabId) =>
    set((s) => ({ runs: withoutKey(s.runs, tabId), messages: withoutKey(s.messages, tabId) })),
}));
