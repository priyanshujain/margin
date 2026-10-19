import { create } from "zustand";
import type { Update } from "@tauri-apps/plugin-updater";

export type UpdatePhase =
  | "idle"
  | "checking"
  | "downloading"
  | "installing"
  | "uptodate"
  | "error";

interface UpdaterState {
  phase: UpdatePhase;
  downloaded: number;
  total: number;
  error: string;
  update: Update | null;
  set: (partial: Partial<UpdaterState>) => void;
  reset: () => void;
}

const initial = {
  phase: "idle" as UpdatePhase,
  downloaded: 0,
  total: 0,
  error: "",
  update: null as Update | null,
};

export const useUpdater = create<UpdaterState>((set) => ({
  ...initial,
  set: (partial) => set(partial),
  reset: () => set(initial),
}));
