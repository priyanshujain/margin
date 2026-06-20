import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Library } from "./components/Library";
import { EditorView } from "./components/EditorView";
import { useBook } from "./store/useBook";
import { isDesktop } from "./ipc";
import { newBook } from "./library";
import { exportEpub, exportPdf } from "./export/exporters";

function App() {
  const book = useBook((s) => s.book);
  const openBook = useBook((s) => s.openBook);

  useEffect(() => {
    if (!isDesktop) return;
    const unlisten = listen<string>("menu-action", (event) => {
      const state = useBook.getState();
      if (event.payload === "new-book") state.openBook(newBook());
      else if (event.payload === "export-pdf" && state.book) exportPdf(state.book);
      else if (event.payload === "export-epub" && state.book) exportEpub(state.book);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  return book ? <EditorView /> : <Library onOpen={openBook} />;
}

export default App;
