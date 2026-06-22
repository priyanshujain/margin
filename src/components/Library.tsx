import { useEffect, useState } from "react";
import { type BookSummary, createAndOpenBook, deleteBook, exampleBook, listBooks, loadBook, saveBook } from "../library";
import type { Book } from "../model/book";
import { importEpub } from "../import/epub";
import { clearPositions } from "../editor/positions";
import { isDesktop } from "../ipc";
import { ConfirmDialog } from "./ConfirmDialog";
import { Icon } from "./Icon";
import { RowMenu } from "./RowMenu";

export function Library({ onOpen }: { onOpen: (book: Book) => void }) {
  const [books, setBooks] = useState<BookSummary[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<BookSummary | null>(null);

  const refresh = () =>
    listBooks()
      .then(setBooks)
      .catch(() => setBooks([]))
      .finally(() => setLoaded(true));
  useEffect(() => {
    refresh();
  }, []);

  const handleExample = async () => {
    const copy = exampleBook();
    if (isDesktop) {
      try {
        await saveBook(copy);
      } catch (e) {
        setNotice(`Could not save copy: ${e}`);
      }
    }
    onOpen(copy);
  };

  const removeBook = async () => {
    if (!pendingDelete) return;
    await deleteBook(pendingDelete.id);
    clearPositions(pendingDelete.id);
    setPendingDelete(null);
    refresh();
  };

  const handleImport = async () => {
    if (!isDesktop) {
      setNotice('Import runs in the desktop app only — open the window from "pnpm tauri dev".');
      return;
    }
    setBusy(true);
    try {
      const book = await importEpub();
      if (!book) return;
      await saveBook(book);
      onOpen(book);
    } catch (e) {
      setNotice(`Import failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="library">
      <header className="library-head" data-tauri-drag-region />
      <div className="shelf">
        <button className="card card-action" onClick={() => createAndOpenBook(onOpen, setNotice)}>
          <Icon d="M12 5v14M5 12h14" size={20} />
          <span>New book</span>
        </button>
        <button className="card card-action" onClick={handleImport} disabled={busy}>
          <Icon d="M12 3v10m0 0l-4-4m4 4l4-4M5 19h14" size={20} />
          <span>{busy ? "Importing…" : "Import EPUB"}</span>
        </button>
        {loaded && books.length === 0 && (
          <button className="card card-example" onClick={handleExample}>
            <span className="card-title">The Lighthouse</span>
            <span className="card-badge">Example</span>
          </button>
        )}
        {books.map((b) =>
          b.corrupt ? (
            <div key={b.id} className="card card-book card-corrupt">
              <span className="card-title">{b.title}</span>
              <span className="card-author">Couldn't be read; a .bak backup may sit beside it.</span>
              <RowMenu label="Book options" className="card-menu" onDelete={() => setPendingDelete(b)} />
            </div>
          ) : (
            <div
              key={b.id}
              className="card card-book"
              onClick={() => loadBook(b.id).then(onOpen).catch((e) => setNotice(`Could not open book: ${e}`))}
            >
              <span className="card-title">{b.title || "Untitled"}</span>
              {b.author && <span className="card-author">{b.author}</span>}
              <RowMenu label="Book options" className="card-menu" onDelete={() => setPendingDelete(b)} />
            </div>
          ),
        )}
      </div>
      {notice && (
        <div className="toast" onClick={() => setNotice(null)}>
          {notice}
        </div>
      )}
      {pendingDelete && (
        <ConfirmDialog
          title="Delete book"
          message={
            <>
              Delete <strong>{pendingDelete.title || "Untitled"}</strong>? This permanently removes the book and all its
              chapters.
            </>
          }
          onConfirm={removeBook}
          onClose={() => setPendingDelete(null)}
        />
      )}
    </div>
  );
}
