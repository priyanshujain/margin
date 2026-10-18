import { create } from "zustand";

export interface OverlayState<O extends string> {
  open: O | null;
  trail: O[];
  show: (overlay: O) => void;
  push: (overlay: O) => void;
  back: () => void;
  reachedFrom: (previous: O) => void;
  toggle: (overlay: O) => void;
  close: () => void;
}

export function createOverlays<O extends string>() {
  return create<OverlayState<O>>((set) => ({
    open: null,
    trail: [],
    show: (overlay) => set({ open: overlay, trail: [] }),
    push: (overlay) =>
      set((s) =>
        s.open === null
          ? { open: overlay, trail: [] }
          : { open: overlay, trail: [...s.trail, s.open] },
      ),
    back: () =>
      set((s) => {
        const trail = s.trail.slice();
        const previous = trail.pop() ?? null;
        return { open: previous, trail };
      }),
    reachedFrom: (previous) =>
      set((s) => (s.open === null || s.open === previous ? {} : { trail: [previous] })),
    toggle: (overlay) =>
      set((s) => (s.open === overlay ? { open: null, trail: [] } : { open: overlay, trail: [] })),
    close: () => set({ open: null, trail: [] }),
  }));
}
