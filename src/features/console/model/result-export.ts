// A result (or part of one) as CSV, TSV, Markdown or JSON, for Copy as and Export.
import { valueText } from "@/features/console/model/cell-format";
import type { ColumnMeta, DbValue } from "@/lib/ipc";

export type ExportFormat = "csv" | "tsv" | "markdown" | "json";

/** What to write: the columns, and the rows already in display order. */
export type ExportSlice = {
  columns: ColumnMeta[];
  /** Each row holds the values of `columns`, in that order. */
  rows: DbValue[][];
};

/** NULL is an empty field in CSV / TSV, the way COPY and spreadsheets expect. */
function fieldText(v: DbValue): string {
  return v.type === "null" ? "" : valueText(v);
}

function delimited(slice: ExportSlice, separator: string): string {
  const quote = (text: string) =>
    text.includes(separator) || /["\r\n]/.test(text) || /^\s|\s$/.test(text)
      ? `"${text.replace(/"/g, '""')}"`
      : text;
  const lines = [slice.columns.map((c) => quote(c.name)).join(separator)];
  for (const row of slice.rows) lines.push(row.map((v) => quote(fieldText(v))).join(separator));
  return `${lines.join("\r\n")}\r\n`;
}

function markdown(slice: ExportSlice): string {
  const cell = (text: string) => text.replace(/\\/g, "\\\\").replace(/\|/g, "\\|").replace(/\r?\n/g, "<br>");
  const header = `| ${slice.columns.map((c) => cell(c.name)).join(" | ")} |`;
  const rule = `| ${slice.columns.map(() => "---").join(" | ")} |`;
  const body = slice.rows.map((row) => `| ${row.map((v) => cell(valueText(v))).join(" | ")} |`);
  return `${[header, rule, ...body].join("\n")}\n`;
}

/** Numbers that fit a JS number stay numbers; bigint and numeric stay strings to keep precision. */
function jsonValue(v: DbValue): unknown {
  switch (v.type) {
    case "null":
      return null;
    case "bool":
    case "int":
      return v.value;
    case "float":
      return v.value; // NaN / Infinity already arrive as null
    case "json":
      return v.value;
    default:
      return valueText(v);
  }
}

/** Repeated column names (e.g. two "id"s from a join) get "_2", "_3" so no key is lost. */
function uniqueKeys(columns: ColumnMeta[]): string[] {
  const seen = new Map<string, number>();
  return columns.map((c) => {
    const n = (seen.get(c.name) ?? 0) + 1;
    seen.set(c.name, n);
    return n === 1 ? c.name : `${c.name}_${n}`;
  });
}

function json(slice: ExportSlice): string {
  const keys = uniqueKeys(slice.columns);
  const objects = slice.rows.map((row) =>
    Object.fromEntries(keys.map((key, i) => [key, jsonValue(row[i] ?? { type: "null" })])),
  );
  return `${JSON.stringify(objects, null, 2)}\n`;
}

export function serialize(slice: ExportSlice, format: ExportFormat): string {
  switch (format) {
    case "csv":
      return delimited(slice, ",");
    case "tsv":
      return delimited(slice, "\t");
    case "markdown":
      return markdown(slice);
    case "json":
      return json(slice);
  }
}

export const FORMAT_LABELS: Record<ExportFormat, string> = {
  csv: "CSV",
  tsv: "TSV",
  markdown: "Markdown",
  json: "JSON",
};

export const FORMAT_EXTENSIONS: Record<ExportFormat, string> = {
  csv: "csv",
  tsv: "tsv",
  markdown: "md",
  json: "json",
};
