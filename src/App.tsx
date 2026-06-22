import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Library } from "./components/Library";
import { EditorView } from "./components/EditorView";
import { useBook } from "./store/useBook";
import { isDesktop } from "./ipc";
import { newBook, saveBook } from "./library";
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
      if (event.payload === "new-book") state.openBook(newBook());
      else if (event.payload === "export-pdf") runExport("pdf");
      else if (event.payload === "export-epub") runExport("epub");
      else if (event.payload === "check-updates") checkForUpdates(false);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    if (!isDesktop) return;
    const win = getCurrentWindow();
    const unlisten = win.onCloseRequested(async (event) => {
      const { book, dirty } = useBook.getState();
      if (!book || !dirty) return;
      event.preventDefault();
      await saveBook(book).catch(() => {});
      win.destroy();
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  return book ? <EditorView /> : <Library onOpen={openBook} />;
}

export default App;
