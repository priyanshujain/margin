import { create } from "zustand";

export type SettingsSectionId = "accounts" | "calendars" | "general";

interface SettingsScreenState {
  open: boolean;
  section: SettingsSectionId;
  show: (section?: SettingsSectionId) => void;
  select: (section: SettingsSectionId) => void;
  close: () => void;
}

/** Settings takes the stage, like Mail's: the grid gives way to it rather than a panel over it. */
export const useSettingsScreen = create<SettingsScreenState>((set) => ({
  open: false,
  section: "accounts",
  show: (section) => set((s) => ({ open: true, section: section ?? s.section })),
  select: (section) => set({ section }),
  close: () => set({ open: false }),
}));
