// Formats for Copy as and Export. The text itself is written by the backend
// (`copyResult` / `exportResult`; src-tauri/src/results/format.rs), so whole
// results can be exported, not only the rows the grid has fetched.
export type ExportFormat = "csv" | "tsv" | "markdown" | "json";

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
