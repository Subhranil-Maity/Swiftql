// Each console tab's CodeMirror view, so controls outside the editor (the Run
// button, the Messages log) can read the cursor and move it.
import type { EditorView } from "@uiw/react-codemirror";

const views = new Map<string, EditorView>();

export function registerEditor(tabId: string, view: EditorView) {
  views.set(tabId, view);
}

/** Forgets the tab's view, unless another one has replaced it already. */
export function unregisterEditor(tabId: string, view: EditorView) {
  if (views.get(tabId) === view) views.delete(tabId);
}

export function editorFor(tabId: string): EditorView | undefined {
  return views.get(tabId);
}
