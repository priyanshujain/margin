import { useEffect, useRef, useState } from "react";
import { COVER_ID, useBook } from "../store/useBook";
import { type Chapter, type ChapterKind, bodyNumber, chapterKind, partNumber, partRoman } from "../model/book";
import { AddPageMenu } from "./AddPageMenu";
import { ConfirmDialog } from "./ConfirmDialog";
import { Icon } from "./Icon";
import { RowMenu } from "./RowMenu";

interface Row {
  chapter: Chapter;
  index: number;
  num: number | null;
  part: number | null;
  indent: boolean;
}

interface DropTarget {
  kind: ChapterKind;
  index: number;
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

export function Sidebar({ onNavigate }: { onNavigate?: () => void }) {
  const chapters = useBook((s) => s.book?.chapters ?? []);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const setActiveChapter = useBook((s) => s.setActiveChapter);
  const addChapter = useBook((s) => s.addChapter);
  const addPage = useBook((s) => s.addPage);
  const addPart = useBook((s) => s.addPart);
  const duplicateChapter = useBook((s) => s.duplicateChapter);
  const setChapterNoTitle = useBook((s) => s.setChapterNoTitle);
  const setChapterNoMargin = useBook((s) => s.setChapterNoMargin);
  const deleteChapter = useBook((s) => s.deleteChapter);
  const moveChapter = useBook((s) => s.moveChapter);
  const closeBook = useBook((s) => s.closeBook);

  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [dropTarget, setDropTarget] = useState<DropTarget | null>(null);
  const [pendingDelete, setPendingDelete] = useState<{ id: string; title: string } | null>(null);
  const [menuOpenId, setMenuOpenId] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());

  const dropRef = useRef<DropTarget | null>(null);
  const gesture = useRef<{ from: number; x: number; y: number; active: boolean } | null>(null);
  const suppressClick = useRef(false);

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 60_000);
    return () => clearInterval(timer);
  }, []);

  const rows: Row[] = chapters.map((chapter, index) => ({
    chapter,
    index,
    num: bodyNumber(chapters, index),
    part: partNumber(chapters, index),
    indent: chapterKind(chapter) === "body" && chapters.slice(0, index).some((c) => chapterKind(c) === "part"),
  }));
  const inBody = (r: Row) => chapterKind(r.chapter) === "body" || chapterKind(r.chapter) === "part";
  const groups: { kind: ChapterKind; label: string; rows: Row[] }[] = [
    { kind: "front", label: "Front matter", rows: rows.filter((r) => chapterKind(r.chapter) === "front") },
    { kind: "body", label: "Chapters", rows: rows.filter(inBody) },
    { kind: "back", label: "Back matter", rows: rows.filter((r) => chapterKind(r.chapter) === "back") },
  ];
  const groupEnd: Record<ChapterKind, number> = {
    front: groups[0].rows.length,
    body: groups[0].rows.length + groups[1].rows.length,
    part: groups[0].rows.length + groups[1].rows.length,
    back: rows.length,
  };

  const dragging = dragIndex !== null;

  const setDrop = (t: DropTarget | null) => {
    dropRef.current = t;
    setDropTarget(t);
  };

  const computeDrop = (x: number, y: number): DropTarget | null => {
    const el = document.elementFromPoint(x, y) as HTMLElement | null;
    const row = el?.closest<HTMLElement>(".chapter");
    if (row?.dataset.idx) {
      const rect = row.getBoundingClientRect();
      const idx = Number(row.dataset.idx);
      const after = y > rect.top + rect.height / 2;
      return { kind: row.dataset.kind as ChapterKind, index: after ? idx + 1 : idx };
    }
    const section = el?.closest<HTMLElement>(".nav-section");
    if (section?.dataset.kind) {
      const kind = section.dataset.kind as ChapterKind;
      return { kind, index: groupEnd[kind] };
    }
    return null;
  };

  const onPointerMove = (e: PointerEvent) => {
    const g = gesture.current;
    if (!g) return;
    if (!g.active) {
      if (Math.abs(e.clientX - g.x) < 4 && Math.abs(e.clientY - g.y) < 4) return;
      g.active = true;
      setDragIndex(g.from);
    }
    const t = computeDrop(e.clientX, e.clientY);
    if (t) setDrop(t);
  };

  const onPointerUp = () => {
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    const g = gesture.current;
    gesture.current = null;
    if (g?.active) {
      suppressClick.current = true;
      const t = dropRef.current;
      if (t) moveChapter(g.from, t.index, t.kind);
    }
    setDragIndex(null);
    setDrop(null);
  };

  const onRowPointerDown = (e: React.PointerEvent, index: number) => {
    if (e.button !== 0 || chapters[index]?.id === menuOpenId || (e.target as HTMLElement).closest(".row-menu-btn")) return;
    suppressClick.current = false;
    gesture.current = { from: index, x: e.clientX, y: e.clientY, active: false };
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
  };

  const onRowClick = (id: string) => {
    if (suppressClick.current) {
      suppressClick.current = false;
      return;
    }
    setActiveChapter(id);
    onNavigate?.();
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
        onClick={() => {
          setActiveChapter(COVER_ID);
          onNavigate?.();
        }}
      >
        <Icon d="M5 4h11l3 3v13H5zM16 4v4h3" size={15} />
        <span className="title">Cover</span>
      </button>
      <AddPageMenu onAdd={addPage} onAddPart={addPart} />

      <div className="nav-scroll">
        {groups.map((group) =>
          group.kind === "body" || group.rows.length || dragging ? (
            <div key={group.kind} className="nav-section" data-kind={group.kind}>
              <div className="nav-label">{group.label}</div>
              <ul className="chapters">
                {group.rows.map((row, i) => {
                  const isPart = chapterKind(row.chapter) === "part";
                  const partLabel = `Part ${partRoman(row.part ?? 0)}`;
                  const rowLabel = isPart
                    ? row.chapter.title
                      ? `${partLabel}: ${row.chapter.title}`
                      : partLabel
                    : row.chapter.title || "Untitled";
                  return (
                  <li
                    key={row.chapter.id}
                    className="chapter"
                    data-idx={row.index}
                    data-kind={group.kind}
                    data-part={isPart}
                    data-indent={row.indent}
                    data-active={row.chapter.id === activeChapterId}
                    data-dragging={dragIndex === row.index}
                    data-drop-before={dropTarget?.kind === group.kind && dropTarget.index === row.index}
                    data-drop-after={dropTarget?.kind === group.kind && i === group.rows.length - 1 && dropTarget.index === row.index + 1}
                    onClick={() => onRowClick(row.chapter.id)}
                    onPointerDown={(e) => onRowPointerDown(e, row.index)}
                  >
                    <span className="grip" title="Drag to reorder">
                      <Icon d="M9 6h.01M15 6h.01M9 12h.01M15 12h.01M9 18h.01M15 18h.01" size={14} />
                    </span>
                    {!isPart && <span className="num">{row.num ?? ""}</span>}
                    <span className="text">
                      {isPart && <span className="part-eyebrow">Part {partRoman(row.part ?? 0)}</span>}
                      {(!isPart || row.chapter.title) && (
                        <span
                          className="title"
                          onMouseEnter={(e) => {
                            const el = e.currentTarget;
                            if (el.scrollWidth > el.clientWidth) el.title = row.chapter.title || "Untitled";
                            else el.removeAttribute("title");
                          }}
                        >
                          {row.chapter.title || "Untitled"}
                        </span>
                      )}
                      {row.chapter.id === activeChapterId && (
                        <span className="meta">Edited {formatEdited(row.chapter.updatedAt, now)}</span>
                      )}
                    </span>
                    <RowMenu
                      label="Page options"
                      onOpenChange={(open) =>
                        setMenuOpenId((cur) => (open ? row.chapter.id : cur === row.chapter.id ? null : cur))
                      }
                      titleHidden={!!row.chapter.noTitle}
                      onToggleTitle={() => setChapterNoTitle(row.chapter.id, !row.chapter.noTitle)}
                      marginHidden={!!row.chapter.noMargin}
                      onToggleMargin={() => setChapterNoMargin(row.chapter.id, !row.chapter.noMargin)}
                      onDuplicate={() => duplicateChapter(row.chapter.id)}
                      onDelete={() => setPendingDelete({ id: row.chapter.id, title: rowLabel })}
                    />
                  </li>
                  );
                })}
                {!group.rows.length && (
                  <li className="chapter-drop" data-on={dropTarget?.kind === group.kind}>
                    Drop here
                  </li>
                )}
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
