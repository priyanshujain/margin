import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ask, message } from "@tauri-apps/plugin-dialog";

let running = false;

export async function checkForUpdates(silent: boolean) {
  if (running) return;
  running = true;
  try {
    const update = await check();
    if (!update) {
      if (!silent) await message("margin is up to date.", { title: "Check for Updates" });
      return;
    }
    const notes = update.body ? `\n\n${update.body}` : "";
    const install = await ask(`margin ${update.version} is available.${notes}\n\nInstall now and restart?`, {
      title: "Update Available",
      kind: "info",
    });
    if (!install) return;
    await update.downloadAndInstall();
    await relaunch();
  } catch (err) {
    if (!silent) {
      await message(`Could not check for updates.\n\n${err}`, { title: "Update Failed", kind: "error" });
    }
  } finally {
    running = false;
  }
}
