import { create } from "zustand";
import { type PreviewMode, initialPreviewMode, persistPreviewMode } from "../devices";

interface PreviewState {
  mode: PreviewMode;
  setMode: (mode: PreviewMode) => void;
}

export const usePreviewMode = create<PreviewState>((set) => ({
  mode: initialPreviewMode(),
  setMode: (mode) => {
    persistPreviewMode(mode);
    set({ mode });
  },
}));
