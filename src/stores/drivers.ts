import { create } from "zustand";

import { api, type DriverInfo } from "@/lib/ipc";
import { errorMessage } from "@/lib/ipc-error";

interface DriversState {
  drivers: DriverInfo[];
  status: "idle" | "loading" | "ready" | "error";
  error: string | null;
  /** Loads once; later calls are no-ops unless the last load failed. */
  load: () => Promise<void>;
}

export const useDriversStore = create<DriversState>()((set, get) => ({
  drivers: [],
  status: "idle",
  error: null,
  load: async () => {
    const { status } = get();
    if (status === "loading" || status === "ready") return;
    set({ status: "loading", error: null });
    try {
      set({ drivers: await api.listDrivers(), status: "ready" });
    } catch (error) {
      set({ status: "error", error: errorMessage(error) });
    }
  },
}));

export function useDriver(id: string | null | undefined): DriverInfo | undefined {
  return useDriversStore((s) => s.drivers.find((d) => d.id === id));
}
