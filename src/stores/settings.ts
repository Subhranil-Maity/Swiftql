import { create } from "zustand";

import { api, events, type Settings } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

/** A partial settings object; the backend deep-merges it. */
export type SettingsPatch = {
  [K in keyof Settings]?: K extends "version" ? never : Partial<Settings[K]>;
};

interface SettingsState {
  settings: Settings | null;
  loaded: boolean;
  error: string | null;
  /** Loads once and starts following `settingsChanged`. */
  load: () => Promise<void>;
  /** Saves `patch` and returns the result. Throws `IpcError` when invalid. */
  update: (patch: SettingsPatch) => Promise<Settings>;
}

let listening = false;

export const useSettingsStore = create<SettingsState>()((set, get) => ({
  settings: null,
  loaded: false,
  error: null,
  load: async () => {
    if (!listening) {
      listening = true;
      void events.settingsChanged.listen((e) => set({ settings: e.payload, error: null }));
    }
    if (get().loaded) return;
    try {
      set({ settings: await api.getSettings(), loaded: true, error: null });
    } catch (error) {
      set({ loaded: true, error: errorMessage(error) });
    }
  },
  update: async (patch) => {
    // A plain JSON object; the backend validates it.
    const settings = await api.updateSettings(patch as Parameters<typeof api.updateSettings>[0]);
    set({ settings, error: null });
    return settings;
  },
}));
