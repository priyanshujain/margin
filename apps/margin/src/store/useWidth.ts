import { create } from "zustand";
import { applyWidth, initialWidth, type Width } from "../width";

interface WidthState {
  width: Width;
  setWidth: (width: Width) => void;
}

export const useWidth = create<WidthState>((set) => ({
  width: initialWidth(),
  setWidth: (width) => {
    applyWidth(width);
    set({ width });
  },
}));
