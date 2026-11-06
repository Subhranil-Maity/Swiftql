import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { useConfirmRunStore } from "@/features/console/stores/confirm-run";

/** Asks before running DROP, TRUNCATE, or DELETE / UPDATE without WHERE. */
export function ConfirmRunDialog() {
  const pending = useConfirmRunStore((s) => s.pending);
  const answer = useConfirmRunStore((s) => s.answer);
  const statements = pending?.statements ?? [];

  return (
    <AlertDialog open={pending !== null} onOpenChange={(open) => !open && answer(false)}>
      <AlertDialogContent className="sm:max-w-lg">
        <AlertDialogHeader>
          <AlertDialogTitle>
            {statements.length === 1 ? "Run a destructive statement?" : `Run ${statements.length} destructive statements?`}
          </AlertDialogTitle>
          <AlertDialogDescription>This can't be undone unless you're inside a transaction you roll back.</AlertDialogDescription>
        </AlertDialogHeader>
        <ul className="flex max-h-60 flex-col gap-2 overflow-auto">
          {statements.map((st, i) => (
            <li key={i} className="rounded-md border bg-muted/40 p-2">
              <p className="mb-1 text-xs font-medium text-destructive">This statement {st.reason}.</p>
              <pre className="font-mono text-xs break-all whitespace-pre-wrap">{st.sql.trim().slice(0, 400)}</pre>
            </li>
          ))}
        </ul>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction variant="destructive" onClick={() => answer(true)}>
            Run anyway
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
