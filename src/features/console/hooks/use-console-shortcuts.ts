import { useEffect, useRef } from "react";

interface ConsoleShortcuts {
  onNewTab: () => void;
  onCloseTab: () => void;
  onCycleTab: (step: 1 | -1) => void;
}

/**
 * Ctrl/Cmd+T opens a tab, Ctrl/Cmd+W closes the active one, and
 * Ctrl+Tab / Ctrl+Shift+Tab cycle through them. Listens on the window so
 * the shortcuts work while typing in the editor.
 */
export function useConsoleShortcuts(handlers: ConsoleShortcuts) {
  // Latest handlers without re-binding the listener on every render.
  const ref = useRef(handlers);
  useEffect(() => {
    ref.current = handlers;
  });

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.altKey) return;
      const mod = event.ctrlKey || event.metaKey;
      if (!mod) return;
      const key = event.key.toLowerCase();

      if (event.key === "Tab" && event.ctrlKey) {
        event.preventDefault();
        ref.current.onCycleTab(event.shiftKey ? -1 : 1);
      } else if (key === "t" && !event.shiftKey) {
        event.preventDefault();
        ref.current.onNewTab();
      } else if (key === "w" && !event.shiftKey) {
        event.preventDefault();
        ref.current.onCloseTab();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
