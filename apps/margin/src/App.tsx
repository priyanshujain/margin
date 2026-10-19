import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Toasts } from "@margin/core/ui";
import { registerCommands, runCommand, useKeymap } from "@margin/core/keys";
import { ProofPopover } from "@margin/core/editor/base";
import { Library } from "./components/Library";
import { EditorView } from "./components/EditorView";
import { SettingsScreen } from "./components/SettingsScreen";
import { ExportPreview } from "./components/ExportPreview";
import { UpdateDialog } from "./components/UpdateDialog";
import { CommandPalette } from "./components/CommandPalette";
import { Shortcuts } from "./components/Shortcuts";
import { useBook } from "./store/useBook";
import { useBackup } from "./store/useBackup";
import { useSettingsScreen } from "./store/useSettingsScreen";
import { useTheme } from "./store/useTheme";
import "./store/useProofing";
import { MENU_COMMANDS, SETTINGS, TOGGLE_THEME } from "./keys/bindings";
import { isDesktop, live } from "./ipc";
import { createAndOpenBook, lastBookId, loadBook, saveBook } from "./library";
import { checkForUpdates } from "./updater";

const listenCorrect = () => () => {};

function App() {
  const book = useBook((s) => s.book);
  const openBook = useBook((s) => s.openBook);
  const settingsOpen = useSettingsScreen((s) => s.open);

  useKeymap();

  useEffect(
    () =>
      registerCommands({
        [SETTINGS]: () => {
          const screen = useSettingsScreen.getState();
          if (screen.open) screen.close();
          else screen.show();
        },
        [TOGGLE_THEME]: () => useTheme.getState().toggle(),
        "new-book": () => {
          const state = useBook.getState();
          void createAndOpenBook(state.openBook, state.setNotice);
        },
        "check-updates": () => void checkForUpdates(false),
        "report-issue": () => void openUrl("https://github.com/priyanshujain/margin/issues").catch(() => {}),
      }),
    [],
  );

  useEffect(() => {
    if (!isDesktop) return;
    checkForUpdates(true);
    const unlisten = listen<string>("menu-action", (event) => {
      const id = MENU_COMMANDS[event.payload];
      if (id) runCommand(id);
    });
    const unlistenWarn = listen<string>("pdf-warnings", (event) => {
      useBook.getState().setNotice(`PDF exported with warnings:\n${event.payload}`);
    });
    return () => {
      unlisten.then((stop) => stop());
      unlistenWarn.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    if (!isDesktop) return;
    const win = getCurrentWindow();
    const unlisten = win.onCloseRequested(async (event) => {
      const { book, dirty } = useBook.getState();
      const { connected } = useBackup.getState();
      if ((!book || !dirty) && !connected) return;
      event.preventDefault();
      if (book && dirty) await saveBook(book).catch(() => {});
      if (connected) useBackup.getState().backup(true);
      win.destroy();
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    if (!isDesktop) return;
    const unlisten = listen<{ ok: boolean; error: string | null; cancelled?: boolean }>("gdrive-auth", (event) => {
      useBackup.getState().handleAuthEvent(event.payload.ok, event.payload.error, event.payload.cancelled ?? false);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    if (!isDesktop) return;
    let first = true;
    const tick = async () => {
      await useBackup.getState().refresh();
      const state = useBackup.getState();
      if (!state.connected || state.needsReauth) return;
      if (first) {
        first = false;
        state.sync(true);
      } else if (state.pending) {
        state.backup(true);
      }
    };
    tick();
    const id = setInterval(tick, 15 * 60 * 1000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    if (!live()) return;
    const id = lastBookId();
    if (!id || useBook.getState().book) return;
    loadBook(id)
      .then((last) => {
        if (!useBook.getState().book) useBook.getState().openBook(last);
      })
      .catch(() => {});
  }, []);

  return (
    <>
      <div hidden={settingsOpen} className="app-root">
        {book ? <EditorView /> : <Library onOpen={openBook} />}
      </div>
      <SettingsScreen />
      {isDesktop && <ExportPreview />}
      {isDesktop && <UpdateDialog />}
      <ProofPopover listen={listenCorrect} />
      <CommandPalette />
      <Shortcuts />
      <Toasts />
    </>
  );
}

export default App;
