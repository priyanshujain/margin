import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Library } from "./components/Library";
import { EditorView } from "./components/EditorView";
import { BackupSettings } from "./components/BackupSettings";
import { ExportPreview } from "./components/ExportPreview";
import { useBook } from "./store/useBook";
import { useBackup } from "./store/useBackup";
import { useExportPreview } from "./store/useExportPreview";
import { isDesktop } from "./ipc";
import { createAndOpenBook, saveBook } from "./library";
import { runExport } from "./export/run";
import { checkForUpdates } from "./updater";

function App() {
  const book = useBook((s) => s.book);
  const openBook = useBook((s) => s.openBook);

  useEffect(() => {
    if (!isDesktop) return;
    checkForUpdates(true);
    const unlisten = listen<string>("menu-action", (event) => {
      const state = useBook.getState();
      if (event.payload === "new-book") createAndOpenBook(state.openBook, state.setNotice);
      else if (event.payload === "export-pdf") {
        if (useBook.getState().book) useExportPreview.getState().openPreview();
      }
      else if (event.payload === "export-epub") runExport("epub");
      else if (event.payload === "check-updates") checkForUpdates(false);
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
      if (connected) {
        await Promise.race([
          useBackup.getState().backup(true),
          new Promise((resolve) => setTimeout(resolve, 8000)),
        ]);
      }
      win.destroy();
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    if (!isDesktop) return;
    const unlisten = listen<{ ok: boolean; error: string | null }>("gdrive-auth", (event) => {
      useBackup.getState().handleAuthEvent(event.payload.ok, event.payload.error);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    if (!isDesktop) return;
    const tick = async () => {
      await useBackup.getState().refresh();
      const state = useBackup.getState();
      if (state.connected && state.pending) state.backup(true);
    };
    tick();
    const id = setInterval(tick, 15 * 60 * 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <>
      {book ? <EditorView /> : <Library onOpen={openBook} />}
      {isDesktop && <BackupSettings />}
      {isDesktop && <ExportPreview />}
    </>
  );
}

export default App;
