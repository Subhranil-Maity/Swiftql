// A pending "run these destructive statements?" question. `confirmRun`
// resolves once the dialog (ConfirmRunDialog) is answered.
import { create } from "zustand";

import type { DestructiveStatement } from "@/features/console/lib/destructive";

interface ConfirmRunState {
  pending: { statements: DestructiveStatement[]; resolve: (ok: boolean) => void } | null;
  answer: (ok: boolean) => void;
}

export const useConfirmRunStore = create<ConfirmRunState>()((set, get) => ({
  pending: null,
  answer: (ok) => {
    get().pending?.resolve(ok);
    set({ pending: null });
  },
}));

export function confirmRun(statements: DestructiveStatement[]): Promise<boolean> {
  // A second question cancels the first.
  useConfirmRunStore.getState().pending?.resolve(false);
  return new Promise((resolve) => useConfirmRunStore.setState({ pending: { statements, resolve } }));
}
