import { type KeyboardEvent, useCallback, useRef, useState } from "react";

/** One visible row of a tree, in display order. */
export type TreeItem = {
  id: string;
  parentId: string | null;
  /** Has children that can be shown or hidden. */
  expandable: boolean;
  expanded: boolean;
};

/**
 * Keyboard support for a `role="tree"` list with a roving tab stop:
 * ↑/↓ move, Home/End jump, → expands or steps into a group, ← collapses or
 * steps out to the parent, and Enter activates (a leaf) or toggles (a group).
 */
export function useListKeyboard({
  items,
  onActivate,
  onToggle,
}: {
  items: TreeItem[];
  onActivate: (id: string) => void;
  onToggle: (id: string, expanded: boolean) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [activeId, setActiveId] = useState<string | null>(null);
  // The tab stop falls back to the first row when the active one was filtered away.
  const current = items.some((i) => i.id === activeId) ? activeId : (items[0]?.id ?? null);

  const focus = useCallback((id: string) => {
    setActiveId(id);
    containerRef.current
      ?.querySelector<HTMLElement>(`[data-tree-id="${CSS.escape(id)}"]`)
      ?.focus();
  }, []);

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.target instanceof HTMLInputElement) return;
    const index = items.findIndex((i) => i.id === current);
    const item = items[index];
    if (!item) return;
    let handled = true;
    switch (event.key) {
      case "ArrowDown":
        if (index < items.length - 1) focus(items[index + 1].id);
        break;
      case "ArrowUp":
        if (index > 0) focus(items[index - 1].id);
        break;
      case "Home":
        focus(items[0].id);
        break;
      case "End":
        focus(items[items.length - 1].id);
        break;
      case "ArrowRight":
        if (item.expandable && !item.expanded) onToggle(item.id, true);
        else if (item.expandable && items[index + 1]?.parentId === item.id) focus(items[index + 1].id);
        break;
      case "ArrowLeft":
        if (item.expandable && item.expanded) onToggle(item.id, false);
        else if (item.parentId) focus(item.parentId);
        break;
      case "Enter":
        if (item.expandable) onToggle(item.id, !item.expanded);
        else onActivate(item.id);
        break;
      default:
        handled = false;
    }
    if (handled) event.preventDefault();
  };

  /** Spread on each row. */
  const itemProps = (id: string) => ({
    "data-tree-id": id,
    tabIndex: id === current ? 0 : -1,
    onFocus: () => setActiveId(id),
  });

  return { containerRef, activeId: current, onKeyDown, itemProps, focus };
}
