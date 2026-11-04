// Each console tab's CodeMirror view, so controls outside the editor (the Run
// button, the Messages log) can read the cursor and move it.
import type { EditorView } from "@uiw/react-codemirror";

const views = new Map<string, EditorView>();
/** Work waiting for a tab's editor to exist, e.g. "Open data" running its query. */
const pending = new Map<string, (view: EditorView) => void>();

export function registerEditor(tabId: string, view: EditorView) {
  views.set(tabId, view);
  const task = pending.get(tabId);
  if (task) {
    pending.delete(tabId);
    // After React has committed the tab, so the run sees a settled editor.
    setTimeout(() => task(view), 0);
  }
}

/** Forgets the tab's view, unless another one has replaced it already. */
export function unregisterEditor(tabId: string, view: EditorView) {
  if (views.get(tabId) === view) views.delete(tabId);
}

export function editorFor(tabId: string): EditorView | undefined {
  return views.get(tabId);
}

/** Calls `task` once the tab's editor is registered, right away if it already is. */
export function whenEditorReady(tabId: string, task: (view: EditorView) => void) {
  const view = views.get(tabId);
  if (view) task(view);
  else pending.set(tabId, task);
}

/** Drops a tab's waiting task, e.g. when the tab closes first. */
export function cancelWhenEditorReady(tabId: string) {
  pending.delete(tabId);
}
