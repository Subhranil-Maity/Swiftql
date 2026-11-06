// How a database value reads in the results grid: close to the text psql
// prints, so what you see is what the server would say.
import { useMemo } from "react";

import type { DbValue, GeneralSettings } from "@/lib/ipc";
import { useSettingsStore } from "@/stores/settings";

export type CellKind = "null" | "number" | "bool" | "text";

export type FormattedCell = {
  /** One line for the grid: newlines are shown as ↵. */
  display: string;
  /** Full text for copying. */
  copy: string;
  kind: CellKind;
};

/** Longest bytea preview, in bytes. */
const BYTES_PREVIEW = 32;

const pad = (n: number, width = 2) => String(Math.abs(n)).padStart(width, "0");

/** "+05:30", "-08", "+00" like Postgres prints UTC offsets. */
function formatOffset(seconds: number): string {
  const sign = seconds < 0 ? "-" : "+";
  const abs = Math.abs(seconds);
  const h = Math.floor(abs / 3600);
  const m = Math.floor((abs % 3600) / 60);
  const s = abs % 60;
  return `${sign}${pad(h)}${m || s ? `:${pad(m)}` : ""}${s ? `:${pad(s)}` : ""}`;
}

/** Postgres interval output (IntervalStyle = postgres): "1 year 2 mons 3 days 04:05:06.5". */
function formatInterval(months: number, days: number, microseconds: number): string {
  const parts: string[] = [];
  const years = Math.trunc(months / 12);
  const mons = months % 12;
  const unit = (n: number, one: string, many: string) => `${n} ${Math.abs(n) === 1 ? one : many}`;
  if (years) parts.push(unit(years, "year", "years"));
  if (mons) parts.push(unit(mons, "mon", "mons"));
  if (days) parts.push(unit(days, "day", "days"));
  if (microseconds || parts.length === 0) {
    const sign = microseconds < 0 ? "-" : "";
    const abs = Math.abs(microseconds);
    const total = Math.floor(abs / 1_000_000);
    const fraction = abs % 1_000_000;
    let time = `${sign}${pad(Math.floor(total / 3600))}:${pad(Math.floor(total / 60) % 60)}:${pad(total % 60)}`;
    if (fraction) time += `.${String(fraction).padStart(6, "0").replace(/0+$/, "")}`;
    parts.push(time);
  }
  return parts.join(" ");
}

/** Array and range elements are quoted when they'd be ambiguous unquoted. */
function quoteElement(text: string, special: RegExp): string {
  if (text === "" || special.test(text) || text.toUpperCase() === "NULL") {
    return `"${text.replace(/["\\]/g, "\\$&")}"`;
  }
  return text;
}

/** The value as Postgres text. */
export function valueText(value: DbValue): string {
  switch (value.type) {
    case "null":
      return "NULL";
    case "bool":
      return value.value ? "true" : "false";
    case "int":
      return String(value.value);
    case "float":
      // NaN and ±Infinity arrive as the text Postgres prints.
      return typeof value.value === "string" ? value.value : value.value === null ? "NaN" : String(value.value);
    case "decimal":
    case "bigInt":
    case "text":
    case "money":
    case "uuid":
    case "date":
    case "time":
      return value.value;
    case "bytes": {
      const hex = value.value.map((b) => b.toString(16).padStart(2, "0")).join("");
      return `\\x${hex}`;
    }
    case "json":
      return JSON.stringify(value.value);
    case "timeTz":
      return `${value.value.time}${formatOffset(value.value.offsetSeconds)}`;
    case "timestamp":
      return value.value.replace("T", " ");
    case "timestampTz":
      // Shown in UTC; the server would use the session's TimeZone.
      return value.value.replace("T", " ").replace(/Z$/, "+00");
    case "interval":
      return formatInterval(value.value.months, value.value.days, value.value.microseconds);
    case "inet": {
      const max = value.value.addr.includes(":") ? 128 : 32;
      return value.value.prefix === max ? value.value.addr : `${value.value.addr}/${value.value.prefix}`;
    }
    case "cidr":
      return `${value.value.addr}/${value.value.prefix}`;
    case "range": {
      const r = value.value;
      if (r.empty) return "empty";
      const bound = (v: DbValue | null) => (v ? quoteElement(valueText(v), /[\s,()[\]"\\]/) : "");
      return `${r.lowerInclusive ? "[" : "("}${bound(r.lower)},${bound(r.upper)}${r.upperInclusive ? "]" : ")"}`;
    }
    case "array":
      return `{${value.value
        .map((v) => (v.type === "null" ? "NULL" : v.type === "array" ? valueText(v) : quoteElement(valueText(v), /[\s,{}"\\]/)))
        .join(",")}}`;
    case "unknown":
      return value.value.text;
  }
}

function kindOf(value: DbValue): CellKind {
  switch (value.type) {
    case "null":
      return "null";
    case "int":
    case "float":
    case "decimal":
    case "bigInt":
      return "number";
    case "bool":
      return "bool";
    default:
      return "text";
  }
}

export function formatCell(value: DbValue): FormattedCell {
  const copy = valueText(value);
  let display = copy;
  if (value.type === "bytes" && value.value.length > BYTES_PREVIEW) {
    display = `${valueText({ type: "bytes", value: value.value.slice(0, BYTES_PREVIEW) })}… (${value.value.length} bytes)`;
  } else if (display.length > 1000) {
    display = `${display.slice(0, 1000)}…`; // the grid shows one line; no need to draw more
  }
  return { display: display.replace(/\r?\n/g, " ↵ "), copy, kind: kindOf(value) };
}

/** How cells look, from Settings → Appearance. */
export type DisplayOptions = Pick<
  GeneralSettings,
  "nullDisplay" | "booleanDisplay" | "timestampDisplay" | "numberGrouping"
>;

const DEFAULT_DISPLAY: DisplayOptions = {
  nullDisplay: "NULL",
  booleanDisplay: "text",
  timestampDisplay: "iso",
  numberGrouping: false,
};

const grouping = new Intl.NumberFormat(undefined, { maximumFractionDigits: 20 });

/** Thousands separators in an integer or decimal string, keeping every digit. */
function groupDigits(text: string): string {
  const match = /^(-?)(\d+)(\.\d+)?$/.exec(text);
  if (!match) return text;
  const [, sign, int, frac = ""] = match;
  const grouped = int.length <= 15 ? grouping.format(Number(int)) : int.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${sign}${grouped}${frac}`;
}

/** The text a cell shows, with the display settings applied; copy text stays as Postgres prints it. */
function displayText(value: DbValue, copy: string, options: DisplayOptions): string {
  switch (value.type) {
    case "null":
      return options.nullDisplay;
    case "bool":
      if (options.booleanDisplay === "short") return value.value ? "t" : "f";
      if (options.booleanDisplay === "symbol") return value.value ? "✓" : "✗";
      return copy;
    case "int":
    case "decimal":
    case "bigInt":
      return options.numberGrouping ? groupDigits(copy) : copy;
    case "float":
      return options.numberGrouping && typeof value.value === "number" && Number.isFinite(value.value)
        ? grouping.format(value.value)
        : copy;
    case "timestampTz":
      if (options.timestampDisplay === "local") {
        const date = new Date(value.value);
        if (!Number.isNaN(date.getTime())) return date.toLocaleString();
      }
      return copy;
    default:
      return copy;
  }
}

/** `formatCell` with display options. */
export function formatCellWith(value: DbValue, options: DisplayOptions): FormattedCell {
  const cell = formatCell(value);
  if (value.type === "bytes" || cell.display.endsWith("…")) return cell; // previews already shortened
  return { ...cell, display: displayText(value, cell.copy, options).replace(/\r?\n/g, " ↵ ") };
}

/** A cell formatter following the current display settings. */
export function useCellFormatter(): (value: DbValue) => FormattedCell {
  const general = useSettingsStore((s) => s.settings?.general);
  const nullDisplay = general?.nullDisplay ?? DEFAULT_DISPLAY.nullDisplay;
  const booleanDisplay = general?.booleanDisplay ?? DEFAULT_DISPLAY.booleanDisplay;
  const timestampDisplay = general?.timestampDisplay ?? DEFAULT_DISPLAY.timestampDisplay;
  const numberGrouping = general?.numberGrouping ?? DEFAULT_DISPLAY.numberGrouping;
  return useMemo(() => {
    const options = { nullDisplay, booleanDisplay, timestampDisplay, numberGrouping };
    return (value: DbValue) => formatCellWith(value, options);
  }, [nullDisplay, booleanDisplay, timestampDisplay, numberGrouping]);
}
