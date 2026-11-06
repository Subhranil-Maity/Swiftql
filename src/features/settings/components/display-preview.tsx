import { formatCellWith } from "@/features/console/model/cell-format";
import type { DbValue, GeneralSettings } from "@/lib/ipc";
import { cn } from "@/lib/utils";

const SAMPLE: { column: string; type: string; value: DbValue }[] = [
  { column: "id", type: "int8", value: { type: "int", value: 1048576 } },
  { column: "active", type: "bool", value: { type: "bool", value: true } },
  { column: "total", type: "numeric", value: { type: "decimal", value: "98234.50" } },
  { column: "shipped_at", type: "timestamptz", value: { type: "timestampTz", value: "2026-03-14T09:26:53Z" } },
  { column: "note", type: "text", value: { type: "null" } },
];

/** A result row drawn with the current display settings, as the grid will show it. */
export function DisplayPreview({ general }: { general: GeneralSettings }) {
  return (
    <figure className="my-2 overflow-hidden rounded-md border bg-background">
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[12.5px]">
          <thead>
            <tr className="border-b bg-muted/40">
              {SAMPLE.map((c) => (
                <th key={c.column} className="px-3 py-1.5 text-left align-bottom font-normal">
                  <span className="block font-medium text-foreground">{c.column}</span>
                  <span className="block font-mono text-[11px] text-muted-foreground">{c.type}</span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            <tr>
              {SAMPLE.map((c) => {
                const cell = formatCellWith(c.value, general);
                return (
                  <td
                    key={c.column}
                    className={cn(
                      "px-3 py-2 font-mono whitespace-nowrap",
                      cell.kind === "number" && "text-right",
                      cell.kind === "null" && "text-muted-foreground italic",
                    )}
                  >
                    {cell.display}
                  </td>
                );
              })}
            </tr>
          </tbody>
        </table>
      </div>
      <figcaption className="border-t px-3 py-1.5 text-xs text-muted-foreground">
        Preview of a result row with these settings.
      </figcaption>
    </figure>
  );
}
