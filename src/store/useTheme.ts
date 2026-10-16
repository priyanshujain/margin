import { create } from "zustand";
import { applyTheme, initialTheme, syncWindowTheme, type Theme } from "../theme";

interface ThemeState {
  theme: Theme;
  toggle: () => void;
}

const startTheme = initialTheme();
syncWindowTheme(startTheme);

export const useTheme = create<ThemeState>((set, get) => ({
  theme: startTheme,
  toggle: () => {
    const next: Theme = get().theme === "dark" ? "light" : "dark";
    applyTheme(next);
    set({ theme: next });
  },
}));
