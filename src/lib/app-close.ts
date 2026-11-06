// Before the window closes: write pending saves (console tabs, graph
// layouts) and close open sessions, so nothing typed in the last moment is lost.
import { getCurrentWindow } from "@tauri-apps/api/window";

import { flushPendingSaves } from "@/lib/debounced-save";
import { useSessionsStore } from "@/stores/sessions";

/** Longest the window waits for sessions to close. */
const CLOSE_SESSIONS_MS = 1500;

export function installCloseHandler() {
  void getCurrentWindow().onCloseRequested(async () => {
    await flushPendingSaves().catch(() => {});
    const { sessions, disconnect } = useSessionsStore.getState();
    const closing = Promise.allSettled(Object.keys(sessions).map((id) => disconnect(id)));
    await Promise.race([closing, new Promise((resolve) => setTimeout(resolve, CLOSE_SESSIONS_MS))]);
    // Not prevented, so the window closes once this resolves.
  });
}
