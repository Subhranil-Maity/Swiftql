// Client-side sorting of a result's rows. Type-aware, stable, and NULLs
// always last, whichever the direction.
import { valueText } from "@/features/console/model/cell-format";
import type { DbValue } from "@/lib/ipc";

export type SortDirection = "asc" | "desc";
export type SortState = { column: number; direction: SortDirection } | null;

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

const isNull = (v: DbValue | undefined) => !v || v.type === "null";

/**
 * Compares decimal strings exactly ("-12.50" vs "3"), with no float rounding.
 * Postgres' numeric NaN sorts above every number, as on the server.
 */
export function compareDecimal(a: string, b: string): number {
  const nanA = a === "NaN";
  const nanB = b === "NaN";
  if (nanA || nanB) return nanA === nanB ? 0 : nanA ? 1 : -1;
  const negA = a.startsWith("-");
  const negB = b.startsWith("-");
  if (negA !== negB) return negA ? -1 : 1;
  const sign = negA ? -1 : 1;
  const [intA, fracA = ""] = a.replace(/^[-+]/, "").split(".");
  const [intB, fracB = ""] = b.replace(/^[-+]/, "").split(".");
  const ia = intA.replace(/^0+(?=\d)/, "");
  const ib = intB.replace(/^0+(?=\d)/, "");
  if (ia.length !== ib.length) return sign * (ia.length - ib.length);
  if (ia !== ib) return sign * (ia < ib ? -1 : 1);
  const width = Math.max(fracA.length, fracB.length);
  const fa = fracA.padEnd(width, "0");
  const fb = fracB.padEnd(width, "0");
  return fa === fb ? 0 : sign * (fa < fb ? -1 : 1);
}

/** Microseconds, roughly: a month counts as 30 days, as Postgres does when comparing. */
function intervalKey(v: { months: number; days: number; microseconds: number }): number {
  return (v.months * 30 + v.days) * 86_400_000_000 + v.microseconds;
}

const NUMERIC = new Set(["int", "float", "decimal", "bigInt"]);

function numericText(v: DbValue): string {
  // A float's NaN arrives as null; "NaN" sorts like numeric NaN.
  if (v.type === "float") return v.value === null ? "NaN" : String(v.value);
  return valueText(v);
}

function isoText(v: DbValue): string | null {
  switch (v.type) {
    case "date":
    case "time":
    case "timestamp":
    case "timestampTz":
      return v.value;
    default:
      return null;
  }
}

/** Two non-NULL values of the same column. */
export function compareValues(a: DbValue, b: DbValue): number {
  if (a.type === "int" && b.type === "int") return a.value - b.value;
  if (a.type === "float" && b.type === "float" && a.value !== null && b.value !== null) {
    return a.value - b.value;
  }
  if (NUMERIC.has(a.type) && NUMERIC.has(b.type)) {
    const ta = numericText(a);
    const tb = numericText(b);
    // Floats can print in exponent form, which the exact compare can't read.
    if (/e/i.test(ta) || /e/i.test(tb)) return Number(ta) - Number(tb);
    return compareDecimal(ta, tb);
  }
  if (a.type === "bool" && b.type === "bool") return Number(a.value) - Number(b.value);
  if (a.type === "interval" && b.type === "interval") return intervalKey(a.value) - intervalKey(b.value);
  // ISO dates and times sort correctly as plain strings.
  const isoA = isoText(a);
  const isoB = isoText(b);
  if (isoA !== null && isoB !== null && a.type === b.type) {
    return isoA === isoB ? 0 : isoA < isoB ? -1 : 1;
  }
  return collator.compare(valueText(a), valueText(b));
}

/** Row indices in sorted order; ties keep their fetched order. */
export function sortRows(rows: DbValue[][], column: number, direction: SortDirection): number[] {
  const order = rows.map((_, i) => i);
  const dir = direction === "asc" ? 1 : -1;
  order.sort((i, j) => {
    const a = rows[i][column];
    const b = rows[j][column];
    const na = isNull(a);
    const nb = isNull(b);
    if (na || nb) return na === nb ? i - j : na ? 1 : -1;
    return dir * compareValues(a, b) || i - j;
  });
  return order;
}
