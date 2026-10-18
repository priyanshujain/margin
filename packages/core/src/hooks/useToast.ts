import { create } from "zustand";

export interface ToastAction {
  label: string;
  keycap?: string;
  run: () => void;
}

export interface ToastState {
  message: string | null;
  action: ToastAction | null;
  seq: number;
  notice: (message: string, action?: ToastAction) => void;
  dismiss: () => void;
}

export const useToast = create<ToastState>((set) => ({
  message: null,
  action: null,
  seq: 0,
  notice: (message, action) =>
    set((s) => ({ message, action: action ?? null, seq: s.seq + 1 })),
  dismiss: () => set({ message: null, action: null }),
}));

export const notify = (message: string, action?: ToastAction) =>
  useToast.getState().notice(message, action);
