// Labels for the console shortcuts. Tab shortcuts are handled in
// `use-console-shortcuts.ts`; the run shortcuts in the editor's keymap (`sql-editor.tsx`).

const isMac = /Mac|iPhone|iPad/.test(navigator.userAgent);
const MOD = isMac ? "⌘" : "Ctrl+";

export const SHORTCUTS = {
  newTab: `${MOD}T`,
  closeTab: `${MOD}W`,
  runStatement: `${MOD}Enter`,
  runScript: isMac ? "⇧⌘Enter" : "Ctrl+Shift+Enter",
} as const;
