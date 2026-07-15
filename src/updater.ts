import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useUpdater } from "./store/useUpdater";

let checking = false;

export async function checkForUpdates(silent: boolean) {
  if (checking) return;
  checking = true;
  const store = useUpdater.getState();
  if (!silent) store.set({ phase: "checking", error: "" });
  try {
    const update = await check();
    if (!update) {
      store.set(silent ? { phase: "idle" } : { phase: "uptodate" });
      return;
    }
    store.set({
      phase: "available",
      update,
      version: update.version,
      notes: update.body ?? "",
      downloaded: 0,
      total: 0,
      error: "",
    });
  } catch (err) {
    store.set(silent ? { phase: "idle" } : { phase: "error", error: String(err) });
  } finally {
    checking = false;
  }
}

export async function installUpdate() {
  const { update, phase } = useUpdater.getState();
  if (!update || phase === "downloading" || phase === "installing") return;
  try {
    useUpdater.getState().set({ phase: "downloading", downloaded: 0, total: 0 });
    await update.downloadAndInstall((event) => {
      const s = useUpdater.getState();
      if (event.event === "Started") {
        s.set({ total: event.data.contentLength ?? 0, downloaded: 0 });
      } else if (event.event === "Progress") {
        s.set({ downloaded: s.downloaded + event.data.chunkLength });
      } else if (event.event === "Finished") {
        s.set({ phase: "installing" });
      }
    });
    useUpdater.getState().set({ phase: "installing" });
    await relaunch();
  } catch (err) {
    useUpdater.getState().set({ phase: "error", error: String(err) });
  }
}

export function dismissUpdate() {
  useUpdater.getState().reset();
}
