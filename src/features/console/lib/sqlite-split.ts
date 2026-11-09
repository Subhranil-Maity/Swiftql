// Statement splitting for SQLite: a small scanner that knows SQLite's
// strings ('…'), quoted names ("…", `…`, […]), comments (-- and /* */) and
// CREATE TRIGGER bodies (BEGIN …; …; END), whose inner `;` don't end the
// statement. It doesn't check syntax; SQLite reports errors when it runs.

/** A statement's [from, to) in UTF-16 offsets, the trailing `;` left out. */
export type SqliteSegment = { from: number; to: number };

const WORD = /[A-Za-z_][A-Za-z0-9_$]*/y;

/** Index just past the quoted run starting at `i` (closer doubled = escaped). */
function skipQuoted(sql: string, i: number, close: string): number {
  let j = i + 1;
  while (j < sql.length) {
    if (sql[j] === close) {
      if (close !== "]" && sql[j + 1] === close) {
        j += 2;
        continue;
      }
      return j + 1;
    }
    j++;
  }
  return sql.length; // unterminated: runs to the end
}

/**
 * Statement ranges of `sql`, untrimmed and without the `;`. Segments with
 * only whitespace and comments are left out.
 */
export function sqliteSegments(sql: string): SqliteSegment[] {
  const segments: SqliteSegment[] = [];
  let start = 0;
  let hasCode = false;
  // Leading keywords of the current statement, to spot CREATE … TRIGGER.
  let leading: string[] = [];
  let trigger = false;
  // Inside a trigger: BEGIN opens the body, CASE … END nests inside it.
  let depth = 0;

  const close = (end: number) => {
    if (hasCode) segments.push({ from: start, to: end });
    hasCode = false;
    leading = [];
    trigger = false;
    depth = 0;
  };

  let i = 0;
  while (i < sql.length) {
    const c = sql[i];
    if (c === "-" && sql[i + 1] === "-") {
      const nl = sql.indexOf("\n", i);
      i = nl < 0 ? sql.length : nl + 1;
      continue;
    }
    if (c === "/" && sql[i + 1] === "*") {
      const endComment = sql.indexOf("*/", i + 2);
      i = endComment < 0 ? sql.length : endComment + 2;
      continue;
    }
    if (c === "'" || c === '"' || c === "`") {
      hasCode = true;
      i = skipQuoted(sql, i, c);
      continue;
    }
    if (c === "[") {
      hasCode = true;
      i = skipQuoted(sql, i, "]");
      continue;
    }
    if (c === ";") {
      if (depth === 0) {
        close(i);
        start = i + 1;
      }
      i++;
      continue;
    }
    WORD.lastIndex = i;
    const word = WORD.exec(sql);
    if (word) {
      hasCode = true;
      const upper = word[0].toUpperCase();
      if (leading.length < 4) {
        leading.push(upper);
        // CREATE [TEMP | TEMPORARY] TRIGGER
        if (upper === "TRIGGER" && leading[0] === "CREATE") trigger = true;
      }
      if (trigger) {
        if (upper === "BEGIN" || (upper === "CASE" && depth > 0)) depth++;
        else if (upper === "END" && depth > 0) depth--;
      }
      i = WORD.lastIndex;
      continue;
    }
    if (!/\s/.test(c)) hasCode = true;
    i++;
  }
  close(sql.length);
  return segments;
}
