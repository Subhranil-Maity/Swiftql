import { useState } from "react";
import { toast } from "sonner";

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
import { Spinner } from "@/components/ui/spinner";
import type { SavedConnection } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";
import { useConnectionsStore } from "@/stores/connections";

export function DeleteConnectionDialog({
  connection,
  onClose,
}: {
  /** The connection to delete; null keeps the dialog closed. */
  connection: SavedConnection | null;
  onClose: () => void;
}) {
  const remove = useConnectionsStore((s) => s.remove);
  const [deleting, setDeleting] = useState(false);

  const onDelete = async () => {
    if (!connection) return;
    setDeleting(true);
    try {
      await remove(connection.id);
      toast.success("Connection deleted");
      onClose();
    } catch (error) {
      toast.error(`Couldn't delete ${connection.name}`, { description: errorMessage(error) });
    } finally {
      setDeleting(false);
    }
  };

  return (
    <AlertDialog open={connection !== null} onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete {connection?.name}?</AlertDialogTitle>
          <AlertDialogDescription>
            This removes the saved connection and its saved password. The database itself
            isn't touched.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={deleting}>Cancel</AlertDialogCancel>
          <AlertDialogAction variant="destructive" onClick={onDelete} disabled={deleting}>
            {deleting && <Spinner data-icon="inline-start" />}
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
