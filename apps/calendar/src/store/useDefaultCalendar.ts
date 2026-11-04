import { create } from "zustand";

const KEY = "margin-calendar.default-calendar";

interface DefaultCalendarState {
  id: string | null;
  set: (id: string | null) => void;
}

export const useDefaultCalendar = create<DefaultCalendarState>((set) => ({
  id: localStorage.getItem(KEY),
  set: (id) => {
    if (id) localStorage.setItem(KEY, id);
    else localStorage.removeItem(KEY);
    set({ id });
  },
}));
