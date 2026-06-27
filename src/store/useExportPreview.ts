import { create } from "zustand";

interface ExportPreviewState {
  open: boolean;
  openPreview: () => void;
  close: () => void;
}

export const useExportPreview = create<ExportPreviewState>((set) => ({
  open: false,
  openPreview: () => set({ open: true }),
  close: () => set({ open: false }),
}));
