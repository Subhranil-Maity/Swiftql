import { FilePlusIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { api } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

/**
 * Creates an empty database file where the user picks in the native "Save
 * as" dialog (the backend opens it), then hands its path back.
 */
export function NewDatabaseButton({ onCreated }: { onCreated: (path: string) => void }) {
  const [busy, setBusy] = useState(false);
  const create = async () => {
    setBusy(true);
    try {
      const path = await api.createDatabaseFile();
      if (path) onCreated(path);
    } catch (error) {
      toast.error("Couldn't create the database", { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  };
  return (
    <Button type="button" variant="outline" disabled={busy} onClick={() => void create()}>
      {busy ? <Spinner /> : <FilePlusIcon />}
      New database…
    </Button>
  );
}
