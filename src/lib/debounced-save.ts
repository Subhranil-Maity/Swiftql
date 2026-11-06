// Debounced saves of UI state (console tabs, graph layouts) to the backend.
// A failed save shows a toast with Retry; `flushPendingSaves` writes every
// pending save at once, e.g. before the window closes.
import { toast } from "sonner";

import { errorMessage } from "@/lib/ipc-error";

type Saver = { flush: () => Promise<void> };

const savers = new Set<Saver>();

/**
 * `save(key)` runs `delay` ms after the last `schedule(key)`. `what` names
 * the thing in the failure toast ("console tabs").
 */
export function createDebouncedSave(
  what: string,
  delay: number,
  save: (key: string) => Promise<unknown> | null,
) {
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  const run = async (key: string) => {
    timers.delete(key);
    const pending = save(key);
    if (!pending) return;
    try {
      await pending;
      toast.dismiss(`save-failed-${what}`);
    } catch (error) {
      console.warn(`saving ${what} (${key}) failed:`, errorMessage(error));
      // One toast per kind of state, replaced by the next failure.
      toast.error(`Couldn't save ${what}`, {
        id: `save-failed-${what}`,
        description: errorMessage(error),
        duration: Infinity,
        action: { label: "Retry", onClick: () => void run(key) },
      });
    }
  };

  const schedule = (key: string) => {
    clearTimeout(timers.get(key));
    timers.set(key, setTimeout(() => void run(key), delay));
  };

  const saver: Saver = {
    flush: async () => {
      const keys = [...timers.keys()];
      for (const key of keys) clearTimeout(timers.get(key));
      await Promise.all(keys.map(run));
    },
  };
  savers.add(saver);

  return { schedule, flush: saver.flush };
}

/** Writes every pending save now. */
export async function flushPendingSaves(): Promise<void> {
  await Promise.all([...savers].map((s) => s.flush()));
}
