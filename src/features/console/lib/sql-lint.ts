// Syntax checking for the SQL editor: red squiggles with the parser's
// message on hover. Postgres scripts go through libpg-query, Postgres's own
// parser compiled to WebAssembly, so valid SQL is never flagged. It reports
// only the first error in a script, like the server does.
import { type Diagnostic, linter } from "@codemirror/lint";
import type { Extension, Text } from "@uiw/react-codemirror";

/** Wait after the last keystroke before re-checking. */
const LINT_DELAY_MS = 400;

type LibPgQuery = typeof import("libpg-query");

// Loaded on first check so the 1.7 MB parser stays out of app startup.
let parser: Promise<LibPgQuery> | null = null;
function loadParser(): Promise<LibPgQuery> {
  parser ??= import("libpg-query").catch((error: unknown) => {
    parser = null; // Try again on the next check.
    throw error;
  });
  return parser;
}

/** Postgres counts characters (code points); CodeMirror offsets are UTF-16 units. */
function codePointToOffset(text: string, codePoints: number): number {
  let offset = 0;
  for (let i = 0; i < codePoints && offset < text.length; i++) {
    offset += (text.codePointAt(offset) ?? 0) > 0xffff ? 2 : 1;
  }
  return offset;
}

/** Range to underline for an error at `pos`: the token there, or the last one at end of input. */
function errorRange(doc: Text, pos: number): { from: number; to: number } {
  const text = doc.toString();
  const written = text.trimEnd();
  if (pos >= written.length) {
    // "syntax error at end of input": mark the last token written.
    const last = /(?:[\p{L}\p{N}_$]+|\S)$/u.exec(written);
    const from = last ? written.length - last[0].length : Math.max(0, written.length - 1);
    return { from, to: Math.max(from + 1, written.length) };
  }
  const token = /^(?:[\p{L}\p{N}_$]+|"[^"]*"?|'[^']*'?|\S)/u.exec(text.slice(pos));
  return { from: pos, to: pos + (token?.[0].length ?? 1) };
}

function capitalize(message: string): string {
  return message.charAt(0).toUpperCase() + message.slice(1);
}

async function checkPostgres(doc: Text): Promise<Diagnostic[]> {
  const sql = doc.toString();
  if (!sql.trim()) return [];

  let pg: LibPgQuery;
  try {
    pg = await loadParser();
  } catch (error) {
    console.warn("SQL syntax checker failed to load:", error);
    return [];
  }

  try {
    await pg.parse(sql);
    return [];
  } catch (error) {
    if (!pg.hasSqlDetails(error)) {
      // Not a syntax error (e.g. the parser ran out of memory): show nothing rather than a wrong squiggle.
      console.warn("SQL syntax check failed:", error);
      return [];
    }
    const pos = codePointToOffset(sql, error.sqlDetails?.cursorPosition ?? 0);
    return [
      {
        ...errorRange(doc, pos),
        severity: "error",
        source: "PostgreSQL",
        message: capitalize(error.message),
      },
    ];
  }
}

/** Syntax checking for a driver's SQL dialect; none for drivers without a parser yet. */
export function sqlLinter(driverId: string): Extension {
  switch (driverId) {
    case "postgres":
      return linter((view) => checkPostgres(view.state.doc), { delay: LINT_DELAY_MS });
    default:
      // TODO: syntax checking for other drivers once they exist
      return [];
  }
}
