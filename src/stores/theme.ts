import { useSyncExternalStore } from "react";
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

function resolveDark(theme: Theme): boolean {
  return theme === "dark" || (theme === "system" && darkQuery.matches);
}

function applyTheme(theme: Theme) {
  document.documentElement.classList.toggle("dark", resolveDark(theme));
}

function subscribeOsTheme(onChange: () => void) {
  darkQuery.addEventListener("change", onChange);
  return () => darkQuery.removeEventListener("change", onChange);
}

/** Whether the app is showing dark mode, following the OS when the theme is "system". */
export function useIsDark(): boolean {
  const theme = useThemeStore((s) => s.theme);
  const osDark = useSyncExternalStore(subscribeOsTheme, () => darkQuery.matches);
  return theme === "dark" || (theme === "system" && osDark);
}

/** Keeps the `.dark` class on <html> in sync with the store and the OS setting. */
export function initTheme() {
  applyTheme(useThemeStore.getState().theme);
  useThemeStore.subscribe((state) => applyTheme(state.theme));
  darkQuery.addEventListener("change", () =>
    applyTheme(useThemeStore.getState().theme),
  );
}
