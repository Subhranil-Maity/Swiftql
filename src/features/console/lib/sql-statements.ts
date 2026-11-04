// Splits a script into the statements Ctrl+Enter and Run script send to the
// server, one at a time.
import { loadPgParser, type PgParser, pgParserIfLoaded } from "@/features/console/lib/pg-parser";
import { utf8ToUtf16 } from "@/features/console/lib/text-offsets";

/** A statement's place in the document, in UTF-16 offsets, without surrounding whitespace. */
export type StatementRange = { from: number; to: number };

/** Narrows [from, to) to its first and last non-whitespace characters; null if blank. */
function trimmed(sql: string, from: number, to: number): StatementRange | null {
  while (from < to && /\s/.test(sql[from])) from++;
  while (to > from && /\s/.test(sql[to - 1])) to--;
  return from < to ? { from, to } : null;
}

const COMMENT_TOKENS = new Set(["SQL_COMMENT", "C_COMMENT"]);

/**
 * Postgres's parser knows exactly where each statement ends, including
 * `BEGIN ATOMIC ... END` bodies. A script with a syntax error doesn't parse,
 * so then the scanner's top-level `;` tokens split it instead.
 */
function splitPostgres(pg: PgParser, sql: string): StatementRange[] {
  if (!sql.trim()) return [];
  const toUtf16 = utf8ToUtf16(sql);
  const ranges: (StatementRange | null)[] = [];
  try {
    for (const stmt of pg.parseSync(sql).stmts ?? []) {
      // stmt_location is left out when 0; stmt_len 0 means "to the end".
      const start = toUtf16(stmt.stmt_location ?? 0);
      const end = stmt.stmt_len ? toUtf16((stmt.stmt_location ?? 0) + stmt.stmt_len) : sql.length;
      ranges.push(trimmed(sql, start, end));
    }
  } catch {
    try {
      let segmentStart = 0;
      let lastEnd = -1; // end of the last code token in this segment; -1 = comments only
      const close = (end: number) => {
        if (lastEnd >= 0) ranges.push(trimmed(sql, segmentStart, end));
      };
      for (const token of pg.scanSync(sql).tokens) {
        if (token.tokenName === "ASCII_59") {
          close(toUtf16(token.start));
          segmentStart = toUtf16(token.end);
          lastEnd = -1;
        } else if (!COMMENT_TOKENS.has(token.tokenName)) {
          lastEnd = toUtf16(token.end);
        }
      }
      close(sql.length);
    } catch {
      // Not even scannable (e.g. a NUL character): send it as written.
      ranges.push(trimmed(sql, 0, sql.length));
    }
  }
  return ranges.filter((r): r is StatementRange => r !== null);
}

/** The whole document as one statement. */
function whole(sql: string): StatementRange[] {
  const range = trimmed(sql, 0, sql.length);
  return range ? [range] : [];
}

/**
 * Statements in `sql`, or null when the splitter for this driver hasn't
 * loaded yet (see `splitStatements`).
 */
export function splitStatementsSync(driverId: string, sql: string): StatementRange[] | null {
  switch (driverId) {
    case "postgres": {
      const pg = pgParserIfLoaded();
      return pg ? splitPostgres(pg, sql) : null;
    }
    default:
      // TODO: statement splitting for other drivers once they exist
      return whole(sql);
  }
}

/** Statements in `sql`, loading the driver's parser if needed. */
export async function splitStatements(driverId: string, sql: string): Promise<StatementRange[]> {
  if (driverId === "postgres") {
    try {
      return splitPostgres(await loadPgParser(), sql);
    } catch (error) {
      console.warn("SQL parser failed to load; running the script as one statement:", error);
      return whole(sql);
    }
  }
  return splitStatementsSync(driverId, sql) ?? whole(sql);
}
