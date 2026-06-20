import { useState } from "react";
import { COVER_ID, useBook } from "../store/useBook";
import { Icon } from "./Icon";

export function Sidebar() {
  const chapters = useBook((s) => s.book?.chapters ?? []);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const setActiveChapter = useBook((s) => s.setActiveChapter);
  const addChapter = useBook((s) => s.addChapter);
  const moveChapter = useBook((s) => s.moveChapter);
  const closeBook = useBook((s) => s.closeBook);

  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [dropIndex, setDropIndex] = useState<number | null>(null);

  const onDragOver = (e: React.DragEvent, i: number) => {
    if (dragIndex === null) return;
    e.preventDefault();
    const rect = e.currentTarget.getBoundingClientRect();
    const after = e.clientY > rect.top + rect.height / 2;
    setDropIndex(after ? i + 1 : i);
  };

  const onDrop = () => {
    if (dragIndex !== null && dropIndex !== null) {
      const to = dragIndex < dropIndex ? dropIndex - 1 : dropIndex;
      moveChapter(dragIndex, to);
    }
    setDragIndex(null);
    setDropIndex(null);
  };

  return (
    <aside className="sidebar">
      <button className="brand" onClick={closeBook} title="All books">
        <Icon d="M14 7l-5 5 5 5" size={15} />
        <span className="back-label">All books</span>
      </button>
      <button
        className="cover-item"
        data-active={activeChapterId === COVER_ID}
        onClick={() => setActiveChapter(COVER_ID)}
      >
        <Icon d="M5 4h11l3 3v13H5zM16 4v4h3" size={15} />
        <span className="title">Cover</span>
      </button>
      <div className="nav-label">Chapters</div>
      <ul className="chapters">
        {chapters.map((chapter, i) => (
          <li
            key={chapter.id}
            className="chapter"
            data-active={chapter.id === activeChapterId}
            data-dragging={dragIndex === i}
            data-drop-before={dropIndex === i}
            data-drop-after={i === chapters.length - 1 && dropIndex === chapters.length}
            draggable
            onClick={() => setActiveChapter(chapter.id)}
            onDragStart={() => setDragIndex(i)}
            onDragOver={(e) => onDragOver(e, i)}
            onDrop={onDrop}
            onDragEnd={() => {
              setDragIndex(null);
              setDropIndex(null);
            }}
          >
            <span className="grip" title="Drag to reorder">
              <Icon d="M9 6h.01M15 6h.01M9 12h.01M15 12h.01M9 18h.01M15 18h.01" size={14} />
            </span>
            <span className="num">{i + 1}</span>
            <span className="title">{chapter.title || "Untitled"}</span>
          </li>
        ))}
      </ul>
      <button className="add-chapter" onClick={addChapter}>
        <Icon d="M12 5v14M5 12h14" />
        New chapter
      </button>
    </aside>
  );
}
