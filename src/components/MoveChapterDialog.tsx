import { useEffect, useRef, useState } from "react";
import type { Chapter } from "../model/book";
import { type BookSummary, listBooks, moveChapterToBook, saveBook } from "../library";
import { useBook } from "../store/useBook";
import { useEscapeLayer } from "../escape";
import { useFocusTrap } from "../focus";
import { Icon } from "./Icon";

interface MoveChapterDialogProps {
  chapter: Chapter;
  label: string;
  onClose: () => void;
}

export function MoveChapterDialog({ chapter, label, onClose }: MoveChapterDialogProps) {
  const bookId = useBook((s) => s.book?.id ?? "");
  const deleteChapter = useBook((s) => s.deleteChapter);
  const setNotice = useBook((s) => s.setNotice);
  const [books, setBooks] = useState<BookSummary[] | null>(null);
  const [moving, setMoving] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  useEscapeLayer(!moving, onClose);
  useFocusTrap(panelRef, books !== null);

  useEffect(() => {
    listBooks()
      .then((list) => setBooks(list.filter((b) => b.id !== bookId && !b.corrupt)))
      .catch(() => setBooks([]));
  }, [bookId]);

  const move = async (target: BookSummary) => {
    setMoving(true);
    try {
      const source = useBook.getState().book;
      if (source) await saveBook(source);
      await moveChapterToBook(chapter, target.id);
      deleteChapter(chapter.id);
      const trimmed = useBook.getState().book;
      if (trimmed) await saveBook(trimmed);
      setNotice(`Moved to ${target.title || "Untitled"}`);
      onClose();
    } catch (e) {
      setNotice(`Could not move: ${e}`);
      setMoving(false);
    }
  };

  return (
    <div className="overlay" onClick={moving ? undefined : onClose}>
      <div ref={panelRef} className="panel panel-move" onClick={(e) => e.stopPropagation()}>
        <div className="panel-head">
          <h2>Move to project</h2>
          <button className="icon-btn" onClick={onClose} title="Close">
            <Icon d="M6 6l12 12M18 6L6 18" />
          </button>
        </div>
        <div className="panel-body">
          <p className="confirm-text">
            Move <strong>{label}</strong> out of this project and into:
          </p>
          {books === null ? (
            <p className="move-empty">Loading projects…</p>
          ) : books.length === 0 ? (
            <p className="move-empty">There is no other project to move this into yet.</p>
          ) : (
            <ul className="move-list">
              {books.map((b) => (
                <li key={b.id}>
                  <button className="move-target" disabled={moving} onClick={() => move(b)}>
                    <span className="move-title">{b.title || "Untitled"}</span>
                    {b.author && <span className="move-author">{b.author}</span>}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="panel-foot">
          <button className="btn-ghost" onClick={onClose} disabled={moving}>
            {moving ? "Moving…" : "Cancel"}
          </button>
        </div>
      </div>
    </div>
  );
}
