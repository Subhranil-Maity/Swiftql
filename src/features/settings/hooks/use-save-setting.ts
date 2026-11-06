import { useCallback } from "react";
import { toast } from "sonner";

import { errorMessage } from "@/lib/ipc-error";
import { type SettingsPatch, useSettingsStore } from "@/stores/settings";

/** Saving one change. `save` toasts failures; `saveOrError` returns the message for inline display. */
export function useSaveSetting() {
  const update = useSettingsStore((s) => s.update);

  const saveOrError = useCallback(
    async (patch: SettingsPatch): Promise<string | null> => {
      try {
        await update(patch);
        return null;
      } catch (error) {
        return errorMessage(error);
      }
    },
    [update],
  );

  const save = useCallback(
    async (patch: SettingsPatch) => {
      const error = await saveOrError(patch);
      if (error) toast.error("Couldn't save the setting", { description: error });
    },
    [saveOrError],
  );

  return { save, saveOrError };
}
