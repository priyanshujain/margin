import { useBook } from "../store/useBook";
import { Icon } from "./Icon";

export function Sidebar() {
  const chapters = useBook((s) => s.book.chapters);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const setActiveChapter = useBook((s) => s.setActiveChapter);
  const addChapter = useBook((s) => s.addChapter);

  return (
    <aside className="sidebar">
      <div className="brand">
        <span className="mark">margin</span>
        <span className="dot" />
      </div>
      <div className="nav-label">Chapters</div>
      <ul className="chapters">
        {chapters.map((chapter, i) => (
          <li
            key={chapter.id}
            className="chapter"
            data-active={chapter.id === activeChapterId}
            onClick={() => setActiveChapter(chapter.id)}
          >
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
