import { useCallback, useState } from "react";
import { toast } from "sonner";

import type { SavedConnection } from "@/lib/ipc";
import { errorMessage, isAuthError, isPasswordRequired } from "@/lib/ipc-error";
import { useSessionsStore } from "@/stores/sessions";

export interface PasswordPrompt {
  connection: SavedConnection;
  /** Set after a wrong password, shown under the field. */
  error: string | null;
  submitting: boolean;
}

/**
 * Connects to a saved connection. When the backend says a password is
 * required, opens a prompt instead of failing; other errors become a toast.
 */
export function useConnect() {
  const connectSession = useSessionsStore((s) => s.connect);
  const connectingId = useSessionsStore((s) => s.connectingId);
  const [prompt, setPrompt] = useState<PasswordPrompt | null>(null);

  const connect = useCallback(
    async (connection: SavedConnection) => {
      try {
        await connectSession(connection.id);
      } catch (error) {
        if (isPasswordRequired(error)) {
          setPrompt({ connection, error: null, submitting: false });
        } else {
          toast.error(`Couldn't connect to ${connection.name}`, {
            description: errorMessage(error),
          });
        }
      }
    },
    [connectSession],
  );

  const submitPassword = useCallback(
    async (password: string) => {
      if (!prompt) return;
      const { connection } = prompt;
      setPrompt({ ...prompt, submitting: true, error: null });
      try {
        await connectSession(connection.id, password);
        setPrompt(null);
      } catch (error) {
        if (isAuthError(error)) {
          setPrompt({ connection, submitting: false, error: errorMessage(error) });
        } else {
          setPrompt(null);
          toast.error(`Couldn't connect to ${connection.name}`, {
            description: errorMessage(error),
          });
        }
      }
    },
    [prompt, connectSession],
  );

  const cancelPrompt = useCallback(() => setPrompt(null), []);

  return { connect, connectingId, prompt, submitPassword, cancelPrompt };
}
