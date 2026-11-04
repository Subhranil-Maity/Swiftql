import { useMemo } from "react";
import type { EditorView } from "@uiw/react-codemirror";

import { editorFor } from "@/features/console/model/editor-registry";
import type { RunError, RunKind } from "@/features/console/model/run";
import { currentStatement, updateStatements } from "@/features/console/lib/statement-markers";
import {
  type StatementRange,
  splitStatements,
  splitStatementsSync,
} from "@/features/console/lib/sql-statements";
import { codePointToOffset } from "@/features/console/lib/text-offsets";
import { api, Channel, type DbError, IpcError, type RunEvent } from "@/lib/ipc";
import { useQueryRunsStore } from "@/stores/query-runs";
import { useSchemaCatalogStore } from "@/stores/schema-catalog";
import { useSchemaModelStore } from "@/stores/schema-model";
import { useSessionsStore } from "@/stores/sessions";

/** Rows fetched per result; the rest is cancelled on the server. */
// TODO: make this a setting, and let the grid load more on demand
export const ROW_LIMIT = 1000;

/** How long to wait for channel events that trail the command's reply. */
const TRAILING_EVENTS_MS = 2000;

type Piece = { sql: string; offset: number };

/** Statements that can add, rename or remove tables and columns. */
// TODO: also catch structure changes made by functions or by DDL rolled back later
const CHANGES_STRUCTURE = /^\s*(?:create|alter|drop|import|comment|select\b[^;]*\binto\b)/i;

async function split(driverId: string, sql: string): Promise<StatementRange[]> {
  return splitStatementsSync(driverId, sql) ?? (await splitStatements(driverId, sql));
}

function pieces(doc: string, ranges: StatementRange[], base = 0): Piece[] {
  return ranges.map((r) => ({ sql: doc.slice(r.from, r.to), offset: base + r.from }));
}

/** The selection with surrounding whitespace removed, or null if it is blank. */
function selectedPiece(view: EditorView): Piece | null {
  const { from, to } = view.state.selection.main;
  const text = view.state.sliceDoc(from, to);
  const lead = text.length - text.trimStart().length;
  const sql = text.trim();
  return sql ? { sql, offset: from + lead } : null;
}

/** What Run statement sends: the selection, else the statement at the cursor. */
async function statementPieces(view: EditorView, driverId: string): Promise<Piece[]> {
  if (!view.state.selection.main.empty) {
    const piece = selectedPiece(view);
    return piece ? [piece] : [];
  }
  const doc = view.state.doc.toString();
  // Split now rather than trust markers that may be up to one debounce behind.
  updateStatements(view, await split(driverId, doc));
  const current = currentStatement(view.state);
  return current ? pieces(view.state.doc.toString(), [current]) : [];
}

/** What Run script sends: every statement, or every statement in the selection. */
async function scriptPieces(view: EditorView, driverId: string): Promise<Piece[]> {
  const { from, to, empty } = view.state.selection.main;
  if (empty) {
    const doc = view.state.doc.toString();
    const ranges = await split(driverId, doc);
    updateStatements(view, ranges);
    return pieces(doc, ranges);
  }
  const text = view.state.sliceDoc(from, to);
  return pieces(text, await split(driverId, text), from);
}

/** A backend error in UI form; `position` counts characters from the statement's start. */
function toRunError(error: DbError | IpcError, view: EditorView | undefined, piece?: Piece): RunError {
  let location: RunError["location"] = null;
  if (view && piece && error.position != null) {
    const offset = piece.offset + codePointToOffset(piece.sql, error.position);
    if (offset <= view.state.doc.length) {
      const line = view.state.doc.lineAt(offset);
      location = { offset, line: line.number, column: offset - line.from + 1 };
    }
  }
  return {
    kind: error.kind,
    message: error.message,
    code: error.code ?? null,
    detail: error.detail ?? null,
    hint: error.hint ?? null,
    location,
  };
}

/** Runs the tab's statement or script on the session; does nothing while the session is busy. */
export async function run(tabId: string, sessionId: string, driverId: string, kind: RunKind) {
  const store = useQueryRunsStore.getState();
  if (store.runningTab[sessionId]) return; // one run per session; the button says so
  const view = editorFor(tabId);
  if (!view) return;

  const list = kind === "script" ? await scriptPieces(view, driverId) : await statementPieces(view, driverId);
  if (list.length === 0 || useQueryRunsStore.getState().runningTab[sessionId]) return;
  store.begin(tabId, sessionId, kind, list);

  // statementFinished / statementFailed events seen, to know when the stream is complete.
  let terminal = 0;
  let onTerminal = () => {};
  const channel = new Channel<RunEvent>();
  channel.onmessage = (event) => {
    let error: RunError | undefined;
    if (event.event === "statementFailed") {
      error = toRunError(event.error, editorFor(tabId), list[event.index]);
    }
    useQueryRunsStore.getState().apply(tabId, event, error);
    if (event.event === "statementFinished" || event.event === "statementFailed") {
      terminal++;
      onTerminal();
    }
  };

  try {
    // The generated binding names the deserialize shape; the wire format is the same.
    const summary = await api.runStatements(
      sessionId,
      list.map((p) => p.sql),
      ROW_LIMIT,
      channel as unknown as Parameters<typeof api.runStatements>[3],
    );
    // The reply can overtake the channel's last messages.
    const expected = summary.executed + (summary.failedAt != null ? 1 : 0);
    if (terminal < expected) {
      await new Promise<void>((resolve) => {
        const timer = setTimeout(resolve, TRAILING_EVENTS_MS);
        onTerminal = () => {
          if (terminal >= expected) {
            clearTimeout(timer);
            resolve();
          }
        };
      });
    }
    useQueryRunsStore.getState().finish(tabId, sessionId, summary);
    useSessionsStore.getState().setTxStatus(sessionId, summary.txStatus);
    // Keep autocomplete and the Tables / Graph views in step with this run's DDL.
    const ran = list.slice(0, summary.executed);
    if (ran.some((p) => CHANGES_STRUCTURE.test(p.sql))) {
      void useSchemaCatalogStore.getState().refresh(sessionId);
      void useSchemaModelStore.getState().refreshIfLoaded(sessionId);
    }
  } catch (error) {
    useQueryRunsStore.getState().fail(tabId, sessionId, toRunError(IpcError.from(error), undefined));
  }
}

/** Stops whatever runs on the session. */
export async function cancelRun(sessionId: string) {
  try {
    await api.cancelRun(sessionId);
  } catch (error) {
    console.warn("cancelling the run failed:", IpcError.from(error).message);
  }
}

/** Running a tab's SQL on its session. */
export function useRunQuery(tabId: string, sessionId: string, driverId: string) {
  const runningTab = useQueryRunsStore((s) => s.runningTab[sessionId]);

  const actions = useMemo(
    () => ({
      runStatement: () => void run(tabId, sessionId, driverId, "statement"),
      runScript: () => void run(tabId, sessionId, driverId, "script"),
      cancel: () => void cancelRun(sessionId),
    }),
    [tabId, sessionId, driverId],
  );

  return {
    ...actions,
    /** This tab is running. */
    running: runningTab === tabId,
    /** Another tab of the same session is running. */
    busyElsewhere: runningTab !== undefined && runningTab !== tabId,
  };
}
