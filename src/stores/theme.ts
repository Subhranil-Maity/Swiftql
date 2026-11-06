// Theme and other appearance settings applied to the page. The theme is
// stored in Settings.general.theme (the backend also reads it to paint the
// window before the webview starts); localStorage keeps a copy only so the
// first paint matches before the settings load.
import { useSyncExternalStore } from "react";
import { create } from "zustand";

import type { Settings } from "@/lib/ipc";
import { useSettingsStore } from "@/stores/settings";

export type Theme = "light" | "dark" | "system";

/** Last theme seen, for the first paint. Older builds stored it here as their only copy. */
const CACHE_KEY = "swiftql-theme";
/** Set once the localStorage theme of older builds was moved into the settings. */
const MIGRATED_KEY = "swiftql-theme-migrated";

function cachedTheme(): Theme {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    // Older builds stored zustand's persist format: {"state":{"theme":"dark"},...}.
    const value = raw?.startsWith("{") ? JSON.parse(raw)?.state?.theme : raw;
    return value === "light" || value === "dark" || value === "system" ? value : "system";
  } catch {
    return "system";
  }
}

interface ThemeState {
  theme: Theme;
  /** Shows the theme at once and saves it to the settings. */
  setTheme: (theme: Theme) => void;
}

export const useThemeStore = create<ThemeState>()((set) => ({
  theme: cachedTheme(),
  setTheme: (theme) => {
    set({ theme });
    useSettingsStore
      .getState()
      .update({ general: { theme } })
      .catch((error: unknown) => console.warn("saving the theme failed:", error));
  },
}));

const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

function resolveDark(theme: Theme): boolean {
  return theme === "dark" || (theme === "system" && darkQuery.matches);
}

function applyTheme(theme: Theme) {
  document.documentElement.classList.toggle("dark", resolveDark(theme));
  localStorage.setItem(CACHE_KEY, theme);
}

/** UI font size and density as CSS on <html>; see index.css. */
function applyAppearance(general: Settings["general"]) {
  const root = document.documentElement;
  root.style.fontSize = `${general.uiFontSize}px`;
  root.dataset.density = general.density;
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

/** Keeps <html> in sync with the theme, the OS setting and the appearance settings. */
export function initTheme() {
  applyTheme(useThemeStore.getState().theme);
  useThemeStore.subscribe((state) => applyTheme(state.theme));
  darkQuery.addEventListener("change", () => applyTheme(useThemeStore.getState().theme));

  useSettingsStore.subscribe((state, previous) => {
    const settings = state.settings;
    if (!settings || settings === previous.settings) return;
    if (!localStorage.getItem(MIGRATED_KEY)) {
      // First start after the theme moved into the settings: keep the user's choice.
      localStorage.setItem(MIGRATED_KEY, "1");
      const old = useThemeStore.getState().theme;
      if (settings.general.theme === "system" && old !== "system") {
        useThemeStore.getState().setTheme(old);
        return;
      }
    }
    if (settings.general.theme !== useThemeStore.getState().theme) {
      useThemeStore.setState({ theme: settings.general.theme });
    }
    applyAppearance(settings.general);
  });
}
