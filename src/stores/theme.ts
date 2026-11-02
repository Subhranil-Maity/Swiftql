import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Theme = "light" | "dark" | "system";

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
}

// TODO: persist the theme via api.getSettings()/api.updateSettings() (src/lib/ipc.ts) instead of localStorage
export const useThemeStore = create<ThemeState>()(
  persist(
    (set) => ({
      theme: "system",
      setTheme: (theme) => set({ theme }),
    }),
    { name: "swiftql-theme" },
  ),
);

const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

function applyTheme(theme: Theme) {
  const isDark = theme === "dark" || (theme === "system" && darkQuery.matches);
  document.documentElement.classList.toggle("dark", isDark);
}

/** Keeps the `.dark` class on <html> in sync with the store and the OS setting. */
export function initTheme() {
  applyTheme(useThemeStore.getState().theme);
  useThemeStore.subscribe((state) => applyTheme(state.theme));
  darkQuery.addEventListener("change", () =>
    applyTheme(useThemeStore.getState().theme),
  );
}
