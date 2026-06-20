import { useCallback, useEffect, useState } from "react";
import type { Editor as TiptapEditor } from "@tiptap/react";
import { Sidebar } from "./components/Sidebar";
import { Dock } from "./components/Dock";
import { Icon } from "./components/Icon";
import { Editor } from "./editor/Editor";
import { FloatingToolbar } from "./editor/FloatingToolbar";
import { useBook } from "./store/useBook";
import { chooseSavePath, openBook, writeBook } from "./project";

function App() {
  const [dock, setDock] = useState(true);
  const [editor, setEditor] = useState<TiptapEditor | null>(null);
  const title = useBook((s) => s.book.metadata.title);
  const author = useBook((s) => s.book.metadata.author);
  const chapters = useBook((s) => s.book.chapters);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const dirty = useBook((s) => s.dirty);
  const setChapterContent = useBook((s) => s.setChapterContent);
  const setChapterTitle = useBook((s) => s.setChapterTitle);
  const replaceBook = useBook((s) => s.replaceBook);
  const markSaved = useBook((s) => s.markSaved);

  const idx = chapters.findIndex((c) => c.id === activeChapterId);
  const chapter = chapters[idx];

  const handleSave = useCallback(async () => {
    const { book, path } = useBook.getState();
    const target = path ?? (await chooseSavePath(book));
    if (!target) return;
    await writeBook(book, target);
    markSaved(target);
  }, [markSaved]);

  const handleOpen = useCallback(async () => {
    const result = await openBook();
    if (result) replaceBook(result.book, result.path);
  }, [replaceBook]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "s") {
        e.preventDefault();
        handleSave();
      }
      if (mod && e.key.toLowerCase() === "o") {
        e.preventDefault();
        handleOpen();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handleSave, handleOpen]);

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <span className="doc-title">
          {title || "Untitled"}
          {author ? ` — ${author}` : ""}
          {dirty && <span className="dirty-dot" />}
        </span>
        <div className="actions">
          <button className="icon-btn" onClick={handleOpen} title="Open (⌘O)">
            <Icon d="M3 7l1.7-2.5h4.6L11 7h8a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z" />
          </button>
          <button className="icon-btn" onClick={handleSave} title="Save (⌘S)">
            <Icon d="M12 4v10m0 0l-3.5-3.5M12 14l3.5-3.5M5 19h14" />
          </button>
          <button className="icon-btn" data-on={dock} onClick={() => setDock(!dock)} title="Toggle preview">
            <Icon d="M3 4.5h18v15H3zM14 4.5v15" />
          </button>
        </div>
      </header>

      <div className="body">
        <Sidebar />
        <main className="editor-pane">
          <article className="sheet">
            <header className="chapter-opener">
              <div className="chapter-num">Chapter {idx + 1}</div>
              <input
                className="chapter-title-input"
                value={chapter.title}
                placeholder="Chapter title"
                spellCheck={false}
                onChange={(e) => setChapterTitle(chapter.id, e.target.value)}
              />
            </header>
            <Editor
              chapterId={chapter.id}
              content={chapter.content}
              onChange={(content) => setChapterContent(chapter.id, content)}
              onReady={setEditor}
            />
          </article>
          <FloatingToolbar editor={editor} />
        </main>
        {dock && <Dock />}
      </div>
    </div>
  );
}

export default App;
