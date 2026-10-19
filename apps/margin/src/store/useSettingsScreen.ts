import { create } from "zustand";

export type SettingsSectionId = "appearance" | "writing" | "backup" | "about";

interface SettingsScreenState {
  open: boolean;
  section: SettingsSectionId;
  show: (section?: SettingsSectionId) => void;
  select: (section: SettingsSectionId) => void;
  close: () => void;
}

export const useSettingsScreen = create<SettingsScreenState>((set) => ({
  open: false,
  section: "appearance",
  show: (section) => set((s) => ({ open: true, section: section ?? s.section })),
  select: (section) => set({ section }),
  close: () => set({ open: false }),
}));
