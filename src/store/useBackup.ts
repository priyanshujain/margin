import { create } from "zustand";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  type BackupStatus,
  gdriveBackup,
  gdriveConnect,
  gdriveDisconnect,
  gdriveRestore,
  gdriveStatus,
  gdriveSync,
} from "../backup";
import { isDesktop } from "../ipc";
import { useBook } from "./useBook";

type Phase = "idle" | "connecting" | "working" | "error";

interface BackupState {
  connected: boolean;
  email: string | null;
  lastBackup: number | null;
  pending: boolean;
  needsReauth: boolean;
  phase: Phase;
  error: string | null;
  authUrl: string | null;
  settingsOpen: boolean;
  restoreNonce: number;
  resolveConnect: ((ok: boolean) => void) | null;
  apply: (status: BackupStatus) => void;
  refresh: () => Promise<void>;
  connect: () => Promise<boolean>;
  cancelConnect: () => void;
  handleAuthEvent: (ok: boolean, error: string | null) => Promise<void>;
  openAuthUrl: () => void;
  copyAuthUrl: () => Promise<void>;
  disconnect: () => Promise<void>;
  backup: (silent?: boolean) => Promise<void>;
  sync: (silent?: boolean) => Promise<void>;
  restore: () => Promise<void>;
  restoreFromDrive: () => Promise<void>;
  openSettings: () => void;
  closeSettings: () => void;
}

function notify(message: string) {
  useBook.getState().setNotice(message);
}

const RECONNECT = /Connect again/;

function describe(error: unknown, fallback: string): string {
  const message = String(error).replace(/^Error:\s*/, "");
  return RECONNECT.test(message) ? message : `${fallback}: ${message}`;
}

function plural(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? "" : "s"}`;
}

function syncMessage(uploaded: number, downloaded: number): string {
  if (!uploaded && !downloaded) return "Everything is already in sync";
  const parts = [];
  if (downloaded) parts.push(`brought back ${plural(downloaded, "file")}`);
  if (uploaded) parts.push(`backed up ${plural(uploaded, "file")}`);
  return `Synced with Google Drive: ${parts.join(", ")}`;
}

export const useBackup = create<BackupState>((set, get) => ({
  connected: false,
  email: null,
  lastBackup: null,
  pending: false,
  needsReauth: false,
  phase: "idle",
  error: null,
  authUrl: null,
  settingsOpen: false,
  restoreNonce: 0,
  resolveConnect: null,
  apply: (status) =>
    set({
      connected: status.connected,
      email: status.email,
      lastBackup: status.lastBackup,
      pending: status.pending,
      needsReauth: status.needsReauth,
    }),
  refresh: async () => {
    if (!isDesktop) return;
    try {
      get().apply(await gdriveStatus());
    } catch {}
  },
  connect: () =>
    new Promise<boolean>((resolve) => {
      get().resolveConnect?.(false);
      set({ phase: "connecting", error: null, authUrl: null, resolveConnect: resolve });
      gdriveConnect()
        .then((url) => set({ authUrl: url }))
        .catch((e) => {
          set({ phase: "error", error: String(e), resolveConnect: null });
          notify(describe(e, "Could not connect"));
          resolve(false);
        });
    }),
  cancelConnect: () => {
    const resolve = get().resolveConnect;
    set({ phase: "idle", authUrl: null, resolveConnect: null });
    resolve?.(false);
  },
  handleAuthEvent: async (ok, error) => {
    if (get().phase !== "connecting") return;
    const resolve = get().resolveConnect;
    if (ok) {
      await get().refresh();
      set({ phase: "idle", authUrl: null, error: null, resolveConnect: null });
      notify("Connected to Google Drive");
      await get().sync();
      resolve?.(true);
      return;
    }
    set({ phase: "error", authUrl: null, error: error ?? "authorization failed", resolveConnect: null });
    notify(describe(error ?? "authorization failed", "Could not connect"));
    resolve?.(false);
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
  disconnect: async () => {
    set({ phase: "working", error: null });
    try {
      get().apply(await gdriveDisconnect());
      set({ phase: "idle" });
      notify("Disconnected from Google Drive");
    } catch (e) {
      set({ phase: "error", error: String(e) });
      notify(`Could not disconnect: ${e}`);
    }
  },
  backup: async (silent = false) => {
    if (!get().connected || get().phase === "working") return;
    set({ phase: "working", error: null });
    try {
      const outcome = await gdriveBackup();
      get().apply(outcome);
      set({ phase: "idle" });
      if (!silent) notify(outcome.uploaded > 0 ? "Backed up to Google Drive" : "Nothing new to back up");
    } catch (e) {
      set({ phase: "error", error: String(e) });
      await get().refresh();
      if (!silent || RECONNECT.test(String(e))) notify(describe(e, "Backup failed"));
    }
  },
  sync: async (silent = false) => {
    if (!get().connected || get().phase === "working") return;
    set({ phase: "working", error: null });
    try {
      const outcome = await gdriveSync();
      get().apply(outcome);
      set((s) => ({ phase: "idle", restoreNonce: outcome.downloaded > 0 ? s.restoreNonce + 1 : s.restoreNonce }));
      if (!silent) notify(syncMessage(outcome.uploaded, outcome.downloaded));
    } catch (e) {
      set({ phase: "error", error: String(e) });
      await get().refresh();
      if (!silent || RECONNECT.test(String(e))) notify(describe(e, "Sync failed"));
    }
  },
  restore: async () => {
    set({ phase: "working", error: null });
    try {
      const result = await gdriveRestore();
      await get().refresh();
      set((s) => ({ phase: "idle", restoreNonce: s.restoreNonce + 1 }));
      notify(result.restored > 0 ? `Restored ${result.restored} file${result.restored === 1 ? "" : "s"}` : "No backups found in Google Drive");
    } catch (e) {
      set({ phase: "error", error: String(e) });
      await get().refresh();
      notify(describe(e, "Restore failed"));
    }
  },
  restoreFromDrive: async () => {
    const ok = get().connected || (await get().connect());
    if (ok) await get().restore();
  },
  openSettings: () => set({ settingsOpen: true }),
  closeSettings: () => set({ settingsOpen: false }),
}));
