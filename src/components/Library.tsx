import { useEffect, useState, type MouseEvent } from "react";
import { type BookSummary, deleteBook, exampleBook, listBooks, loadBook, newBook } from "../library";
import type { Book } from "../model/book";
import { Icon } from "./Icon";

export function Library({ onOpen }: { onOpen: (book: Book) => void }) {
  const [books, setBooks] = useState<BookSummary[]>([]);

  const refresh = () => listBooks().then(setBooks).catch(() => setBooks([]));
  useEffect(() => {
    refresh();
  }, []);

  const removeBook = async (e: MouseEvent, id: string) => {
    e.stopPropagation();
    await deleteBook(id);
    refresh();
  };

  return (
    <div className="library">
      <header className="library-head">
        <span className="mark">margin</span>
        <span className="dot" />
      </header>
      <div className="shelf">
        <button className="card card-action" onClick={() => onOpen(newBook())}>
          <Icon d="M12 5v14M5 12h14" size={20} />
          <span>New book</span>
        </button>
        <button className="card card-example" onClick={() => onOpen(exampleBook())}>
          <span className="card-title">The Lighthouse</span>
          <span className="card-badge">Example</span>
        </button>
        {books.map((b) => (
          <div key={b.id} className="card card-book" onClick={() => loadBook(b.id).then(onOpen)}>
            <span className="card-title">{b.title || "Untitled"}</span>
            {b.author && <span className="card-author">{b.author}</span>}
            <button className="card-delete" title="Delete book" onClick={(e) => removeBook(e, b.id)}>
              <Icon d="M6 6l12 12M18 6L6 18" size={13} />
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
