// Labels for the console shortcuts handled in `use-console-shortcuts.ts`.

const isMac = /Mac|iPhone|iPad/.test(navigator.userAgent);
const MOD = isMac ? "⌘" : "Ctrl+";

export const SHORTCUTS = {
  newTab: `${MOD}T`,
  closeTab: `${MOD}W`,
  // TODO: wire up once queries can run
  run: `${MOD}Enter`,
} as const;
