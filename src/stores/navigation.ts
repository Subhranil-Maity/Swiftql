import { create } from "zustand";

/** Which page shows while no session is open. */
export type View =
  /** `connectId` connects to that saved connection on arrival ("Save and connect"). */
  | { page: "welcome"; connectId?: string }
  /** `editingId` is the saved connection being edited; null for a new one. */
  | { page: "connection"; editingId: string | null }
  | { page: "settings" };

interface NavigationState {
  view: View;
  go: (view: View) => void;
}

export const useNavigationStore = create<NavigationState>()((set) => ({
  view: { page: "welcome" },
  go: (view) => set({ view }),
}));
