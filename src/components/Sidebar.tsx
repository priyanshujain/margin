import { useEffect, useState } from "react";
import { COVER_ID, useBook } from "../store/useBook";
import { type Chapter, type ChapterKind, bodyNumber, chapterKind } from "../model/book";
import { AddPageMenu } from "./AddPageMenu";
import { ConfirmDialog } from "./ConfirmDialog";
import { Icon } from "./Icon";
import { RowMenu } from "./RowMenu";

interface Row {
  chapter: Chapter;
  index: number;
  num: number | null;
}

function formatEdited(ts: number, now: number): string {
  const diff = Math.max(0, now - ts);
  const min = 60_000;
  const hour = 60 * min;
  const day = 24 * hour;
  if (diff < min) return "just now";
  if (diff < hour) {
    const m = Math.floor(diff / min);
    return `${m} minute${m === 1 ? "" : "s"} ago`;
  }
  if (diff < day) {
    const h = Math.floor(diff / hour);
    return `${h} hour${h === 1 ? "" : "s"} ago`;
  }
  if (diff < 2 * day) return "yesterday";
  const d = Math.floor(diff / day);
  if (d < 7) return `${d} days ago`;
  return new Date(ts).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

export function Sidebar() {
  const chapters = useBook((s) => s.book?.chapters ?? []);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const setActiveChapter = useBook((s) => s.setActiveChapter);
  const addChapter = useBook((s) => s.addChapter);
  const addPage = useBook((s) => s.addPage);
  const deleteChapter = useBook((s) => s.deleteChapter);
  const moveChapter = useBook((s) => s.moveChapter);
  const closeBook = useBook((s) => s.closeBook);

  const [drag, setDrag] = useState<{ kind: ChapterKind; index: number } | null>(null);
  const [dropIndex, setDropIndex] = useState<number | null>(null);
  const [pendingDelete, setPendingDelete] = useState<{ id: string; title: string } | null>(null);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 60_000);
    return () => clearInterval(timer);
  }, []);

  const rows: Row[] = chapters.map((chapter, index) => ({ chapter, index, num: bodyNumber(chapters, index) }));
  const groups: { kind: ChapterKind; label: string; rows: Row[] }[] = [
    { kind: "front", label: "Front matter", rows: rows.filter((r) => chapterKind(r.chapter) === "front") },
    { kind: "body", label: "Chapters", rows: rows.filter((r) => chapterKind(r.chapter) === "body") },
    { kind: "back", label: "Back matter", rows: rows.filter((r) => chapterKind(r.chapter) === "back") },
  ];

  const onDragOver = (e: React.DragEvent, kind: ChapterKind, index: number) => {
    if (!drag || drag.kind !== kind) return;
    e.preventDefault();
    const rect = e.currentTarget.getBoundingClientRect();
    const after = e.clientY > rect.top + rect.height / 2;
    setDropIndex(after ? index + 1 : index);
  };

  const onDrop = () => {
    if (drag && dropIndex !== null) {
      const to = drag.index < dropIndex ? dropIndex - 1 : dropIndex;
      moveChapter(drag.index, to);
    }
    setDrag(null);
    setDropIndex(null);
  };

  const reset = () => {
    setDrag(null);
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
      <AddPageMenu onAdd={addPage} />

      <div className="nav-scroll">
        {groups.map((group) =>
          group.kind === "body" || group.rows.length ? (
            <div key={group.kind} className="nav-section">
              <div className="nav-label">{group.label}</div>
              <ul className="chapters">
                {group.rows.map((row, i) => (
                  <li
                    key={row.chapter.id}
                    className="chapter"
                    data-active={row.chapter.id === activeChapterId}
                    data-dragging={drag?.index === row.index}
                    data-drop-before={drag?.kind === group.kind && dropIndex === row.index}
                    data-drop-after={drag?.kind === group.kind && i === group.rows.length - 1 && dropIndex === row.index + 1}
                    draggable
                    onClick={() => setActiveChapter(row.chapter.id)}
                    onDragStart={() => setDrag({ kind: group.kind, index: row.index })}
                    onDragOver={(e) => onDragOver(e, group.kind, row.index)}
                    onDrop={onDrop}
                    onDragEnd={reset}
                  >
                    <span className="grip" title="Drag to reorder">
                      <Icon d="M9 6h.01M15 6h.01M9 12h.01M15 12h.01M9 18h.01M15 18h.01" size={14} />
                    </span>
                    <span className="num">{row.num ?? ""}</span>
                    <span className="text">
                      <span className="title">{row.chapter.title || "Untitled"}</span>
                      {row.chapter.id === activeChapterId && (
                        <span className="meta">Edited {formatEdited(row.chapter.updatedAt, now)}</span>
                      )}
                    </span>
                    <RowMenu
                      label="Page options"
                      onDelete={() => setPendingDelete({ id: row.chapter.id, title: row.chapter.title || "Untitled" })}
                    />
                  </li>
                ))}
              </ul>
              {group.kind === "body" && (
                <button className="add-chapter" onClick={addChapter}>
                  <Icon d="M12 5v14M5 12h14" />
                  New chapter
                </button>
              )}
            </div>
          ) : null
        )}
      </div>

      {pendingDelete && (
        <ConfirmDialog
          title="Delete"
          message={
            <>
              Delete <strong>{pendingDelete.title}</strong>? This can't be undone.
            </>
          }
          onConfirm={() => {
            deleteChapter(pendingDelete.id);
            setPendingDelete(null);
          }}
          onClose={() => setPendingDelete(null)}
        />
      )}
    </aside>
  );
}
