import { useMemo } from "react";
import { CopyIcon } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { valueText } from "@/features/console/model/cell-format";
import type { ColumnMeta, DbValue } from "@/lib/ipc";

export type ViewedCell = { column: ColumnMeta; value: DbValue; row: number };

/** JSON pretty-printed (json columns, and text that holds an object or array); anything else as is. */
function fullText(value: DbValue): { text: string; json: boolean } {
  if (value.type === "json") return { text: JSON.stringify(value.value, null, 2), json: true };
  const text = valueText(value);
  if (value.type === "text" && /^\s*[[{]/.test(text)) {
    try {
      return { text: JSON.stringify(JSON.parse(text), null, 2), json: true };
    } catch {
      // Not JSON after all.
    }
  }
  return { text, json: false };
}

/** A cell's whole value: double-click or Enter on a cell opens it. */
export function CellViewerDialog({ cell, onClose }: { cell: ViewedCell | null; onClose: () => void }) {
  const shown = useMemo(() => (cell ? fullText(cell.value) : null), [cell]);

  const copy = async () => {
    if (!shown) return;
    try {
      await navigator.clipboard.writeText(shown.text);
      toast.success("Copied value");
    } catch {
      toast.error("Couldn't copy to the clipboard");
    }
  };

  return (
    <Dialog open={cell !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="flex max-h-[80vh] flex-col sm:max-w-2xl">
        {cell && shown && (
          <>
            <DialogHeader>
              <DialogTitle>{cell.column.name}</DialogTitle>
              <DialogDescription>
                <span className="font-mono text-xs">{cell.column.nativeType}</span> · row{" "}
                {(cell.row + 1).toLocaleString()}
                {shown.json && " · formatted as JSON"}
              </DialogDescription>
            </DialogHeader>
            {cell.value.type === "null" ? (
              <p className="text-muted-foreground italic">NULL</p>
            ) : (
              <pre className="min-h-0 flex-1 overflow-auto rounded-md border bg-muted/40 p-3 font-mono text-xs whitespace-pre-wrap break-all select-text">
                {shown.text}
              </pre>
            )}
            <DialogFooter>
              <Button variant="outline" size="sm" onClick={() => void copy()} disabled={cell.value.type === "null"}>
                <CopyIcon data-icon="inline-start" />
                Copy
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
