import { create } from "zustand";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  accountConnect,
  accountConnectCancel,
  accountDisconnect,
  accountsList,
  caldavConnect,
} from "../api/accounts";
import {
  calendarCreate,
  calendarDelete,
  calendarSetSelected,
  calendarsList,
  icsImport,
  icsSubscribe,
} from "../api/calendars";
import { live, type Account, type Calendar } from "../ipc";
import { notify } from "@margin/core/hooks";

type Phase = "idle" | "connecting" | "working" | "error";

interface AccountsState {
  accounts: Account[];
  calendars: Calendar[];
  /**
   * True once a list has actually come back. An empty `accounts` before that is a page that has not
   * asked yet, and telling the two apart is the difference between "connect one" and a first frame
   * of it on every launch.
   */
  loaded: boolean;
  phase: Phase;
  error: string | null;
  authUrl: string | null;
  resolveConnect: ((ok: boolean) => void) | null;
  refresh: () => Promise<void>;
  connect: () => Promise<boolean>;
  cancelConnect: () => void;
  handleAuthEvent: (ok: boolean, error: string | null, cancelled?: boolean) => Promise<void>;
  openAuthUrl: () => void;
  copyAuthUrl: () => Promise<void>;
  disconnect: (accountId: string) => Promise<void>;
  connectCalDav: (server: string, username: string, password: string) => Promise<boolean>;
  setSelected: (calendarId: string, selected: boolean) => Promise<void>;
  createCalendar: (name: string) => Promise<boolean>;
  deleteCalendar: (calendarId: string) => Promise<void>;
  subscribe: (url: string) => Promise<boolean>;
  importFile: (file: File) => Promise<void>;
}

export const useAccounts = create<AccountsState>((set, get) => ({
  accounts: [],
  calendars: [],
  loaded: false,
  phase: "idle",
  error: null,
  authUrl: null,
  resolveConnect: null,
  refresh: async () => {
    if (!live()) return;
    try {
      const [accounts, calendars] = await Promise.all([accountsList(), calendarsList()]);
      set({ accounts, calendars, loaded: true });
    } catch (e) {
      set({ error: String(e) });
    }
  },
  // The promise holds its own resolver so a later Tauri `auth` event can settle it. Ported from
  // margin/src/store/useBackup.ts:70.
  connect: () =>
    new Promise<boolean>((resolve) => {
      get().resolveConnect?.(false);
      set({ phase: "connecting", error: null, authUrl: null, resolveConnect: resolve });
      accountConnect()
        .then((url) => set({ authUrl: url }))
        .catch((e) => {
          set({ phase: "error", error: String(e), resolveConnect: null });
          resolve(false);
        });
    }),
  cancelConnect: () => {
    const resolve = get().resolveConnect;
    set({ phase: "idle", authUrl: null, error: null, resolveConnect: null });
    void accountConnectCancel().catch(() => {});
    resolve?.(false);
  },
  handleAuthEvent: async (ok, error, cancelled = false) => {
    if (get().phase !== "connecting") return;
    const resolve = get().resolveConnect;
    if (ok) {
      await get().refresh();
      set({ phase: "idle", authUrl: null, error: null, resolveConnect: null });
      notify("Google account added");
    } else if (cancelled) {
      // Closing the consent browser is an answer, not a fault. Back to idle with nothing said:
      // the user already knows what they did, and a red panel telling them about it reads as
      // though shutting the sheet broke something.
      set({ phase: "idle", authUrl: null, error: null, resolveConnect: null });
    } else {
      const message = error ?? "authorization failed";
      set({ phase: "error", authUrl: null, error: message, resolveConnect: null });
    }
    resolve?.(ok);
  },
  openAuthUrl: () => {
    const url = get().authUrl;
    if (url) openUrl(url).catch(() => {});
  },
  copyAuthUrl: async () => {
    const url = get().authUrl;
    if (!url) return;
    try {
      await navigator.clipboard.writeText(url);
      notify("Authorization link copied");
    } catch {
      notify("Could not copy the link");
    }
  },
  disconnect: async (accountId) => {
    set({ phase: "working", error: null });
    try {
      await accountDisconnect(accountId);
      await get().refresh();
      set({ phase: "idle" });
      notify("Account removed");
    } catch (e) {
      set({ phase: "error", error: String(e) });
      notify(`Could not disconnect that account: ${e}`);
    }
  },
  connectCalDav: async (server, username, password) => {
    set({ phase: "working", error: null });
    try {
      const account = await caldavConnect(server, username, password);
      await get().refresh();
      set({ phase: "idle" });
      notify(`Connected ${account.email}`);
      return true;
    } catch (e) {
      set({ phase: "error", error: String(e) });
      return false;
    }
  },
  setSelected: async (calendarId, selected) => {
    set((s) => ({
      calendars: s.calendars.map((c) => (c.id === calendarId ? { ...c, selected } : c)),
    }));
    try {
      await calendarSetSelected(calendarId, selected);
    } catch (e) {
      await get().refresh();
      notify(`Could not update the calendar: ${e}`);
    }
  },
  createCalendar: async (name) => {
    try {
      await calendarCreate(name);
      await get().refresh();
      return true;
    } catch (e) {
      notify(`Could not create the calendar: ${e}`);
      return false;
    }
  },
  subscribe: async (url) => {
    try {
      const calendar = await icsSubscribe(url);
      await get().refresh();
      notify(`Subscribed to “${calendar.summary}”`);
      return true;
    } catch (e) {
      notify(`Could not subscribe: ${e}`);
      return false;
    }
  },
  importFile: async (file) => {
    try {
      const text = await file.text();
      const name = file.name.replace(/\.(ics|ical|ifb|icalendar)$/i, "") || "Imported";
      const { calendar, events } = await icsImport(text, name);
      await get().refresh();
      notify(`Imported ${events} ${events === 1 ? "event" : "events"} into “${calendar.summary}”`);
    } catch (e) {
      notify(`Could not import that file: ${e}`);
    }
  },
  deleteCalendar: async (calendarId) => {
    try {
      await calendarDelete(calendarId);
      await get().refresh();
    } catch (e) {
      notify(`Could not delete the calendar: ${e}`);
    }
  },
}));
