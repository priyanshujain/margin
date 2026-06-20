import { useState } from "react";
import type { Editor as TiptapEditor } from "@tiptap/react";
import { Sidebar } from "./components/Sidebar";
import { Dock } from "./components/Dock";
import { Icon } from "./components/Icon";
import { Editor } from "./editor/Editor";
import { FloatingToolbar } from "./editor/FloatingToolbar";
import { useBook } from "./store/useBook";

function App() {
  const [dock, setDock] = useState(true);
  const [editor, setEditor] = useState<TiptapEditor | null>(null);
  const title = useBook((s) => s.book.metadata.title);
  const author = useBook((s) => s.book.metadata.author);
  const chapters = useBook((s) => s.book.chapters);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const setChapterContent = useBook((s) => s.setChapterContent);
  const setChapterTitle = useBook((s) => s.setChapterTitle);

  const idx = chapters.findIndex((c) => c.id === activeChapterId);
  const chapter = chapters[idx];

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <span className="doc-title">
          {title || "Untitled"}
          {author ? ` — ${author}` : ""}
        </span>
        <div className="actions">
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
