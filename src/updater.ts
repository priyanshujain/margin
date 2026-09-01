import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ask } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useUpdater } from "./store/useUpdater";

const DECLINED = "margin.update.declined";

type Channel = "direct" | "appstore" | "none";

let checking = false;
let channel: Channel | null = null;

async function updateChannel() {
  if (!channel) channel = await invoke<Channel>("update_channel");
  return channel;
}

function declined(version: string) {
  return localStorage.getItem(DECLINED) === version;
}

function decline(version: string) {
  localStorage.setItem(DECLINED, version);
}

function offer(version: string, current: string, okLabel: string) {
  return ask(`margin ${version} is available. You have ${current}.`, {
    title: "A new version of margin is available",
    kind: "info",
    okLabel,
    cancelLabel: "Later",
  });
}

export async function checkForUpdates(silent: boolean) {
  if (checking) return;
  checking = true;
  const store = useUpdater.getState();
  if (!silent) store.set({ phase: "checking", error: "" });
  try {
    const target = await updateChannel();
    if (target === "direct") await checkDirect(silent);
    else if (target === "appstore") await checkAppStore(silent);
    else store.set(silent ? { phase: "idle" } : { phase: "uptodate" });
  } catch (err) {
    store.set(silent ? { phase: "idle" } : { phase: "error", error: String(err) });
  } finally {
    checking = false;
  }
}

async function checkDirect(silent: boolean) {
  const store = useUpdater.getState();
  const update = await check();
  if (!update) {
    store.set(silent ? { phase: "idle" } : { phase: "uptodate" });
    return;
  }
  if (silent && declined(update.version)) {
    store.set({ phase: "idle" });
    return;
  }
  store.set({ phase: "idle", update });
  if (await offer(update.version, update.currentVersion, "Install Update")) await installUpdate();
  else decline(update.version);
}

async function checkAppStore(silent: boolean) {
  const store = useUpdater.getState();
  const release = await invoke<{ version: string; trackId: number } | null>("appstore_latest");
  if (!release) {
    store.set(silent ? { phase: "idle" } : { phase: "uptodate" });
    return;
  }
  if (silent && declined(release.version)) {
    store.set({ phase: "idle" });
    return;
  }
  store.set({ phase: "idle" });
  if (await offer(release.version, await getVersion(), "Open App Store"))
    await invoke("open_appstore", { trackId: release.trackId });
  else decline(release.version);
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
