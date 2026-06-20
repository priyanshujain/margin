import { useCallback, useEffect, useState } from "react";
import type { Editor as TiptapEditor } from "@tiptap/react";
import { Sidebar } from "./components/Sidebar";
import { Dock } from "./components/Dock";
import { Icon } from "./components/Icon";
import { Settings } from "./components/Settings";
import { Editor } from "./editor/Editor";
import { FloatingToolbar } from "./editor/FloatingToolbar";
import { useBook } from "./store/useBook";
import type { Book } from "./model/book";
import { chooseSavePath, openBook, writeBook } from "./project";
import { exportEpub, exportPdf } from "./export/exporters";
import { isDesktop } from "./ipc";

const DESKTOP_ONLY =
  'This runs in the desktop app only — open the window from "pnpm tauri dev" (you are viewing the browser preview).';

function App() {
  const [dock, setDock] = useState(true);
  const [editor, setEditor] = useState<TiptapEditor | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
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
    if (!isDesktop) {
      setNotice(DESKTOP_ONLY);
      return;
    }
    try {
      const { book, path } = useBook.getState();
      const target = path ?? (await chooseSavePath(book));
      if (!target) return;
      await writeBook(book, target);
      markSaved(target);
      setNotice("Saved");
    } catch (e) {
      setNotice(`Save failed: ${e}`);
    }
  }, [markSaved]);

  const handleOpen = useCallback(async () => {
    if (!isDesktop) {
      setNotice(DESKTOP_ONLY);
      return;
    }
    try {
      const result = await openBook();
      if (result) replaceBook(result.book, result.path);
    } catch (e) {
      setNotice(`Open failed: ${e}`);
    }
  }, [replaceBook]);

  const runExport = async (label: string, fn: (book: Book) => Promise<void>) => {
    setExportOpen(false);
    if (!isDesktop) {
      setNotice(DESKTOP_ONLY);
      return;
    }
    try {
      await fn(useBook.getState().book);
    } catch (e) {
      setNotice(`${label} export failed: ${e}`);
    }
  };

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

  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(null), 4000);
    return () => clearTimeout(timer);
  }, [notice]);

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <button className="doc-title" onClick={() => setSettingsOpen(true)} title="Book setup">
          {title || "Untitled"}
          {author ? ` — ${author}` : ""}
          {dirty && <span className="dirty-dot" />}
        </button>
        <div className="actions">
          <button className="icon-btn" onClick={handleOpen} title="Open (⌘O)">
            <Icon d="M3 7l1.7-2.5h4.6L11 7h8a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z" />
          </button>
          <button className="icon-btn" onClick={handleSave} title="Save (⌘S)">
            <Icon d="M12 4v10m0 0l-3.5-3.5M12 14l3.5-3.5M5 19h14" />
          </button>
          <div className="menu-wrap">
            <button className="icon-btn" data-on={exportOpen} onClick={() => setExportOpen((v) => !v)} title="Export">
              <Icon d="M12 15V4m0 0L8 8m4-4l4 4M5 14v5a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-5" />
            </button>
            {exportOpen && (
              <>
                <div className="menu-backdrop" onClick={() => setExportOpen(false)} />
                <div className="menu">
                  <button onClick={() => runExport("PDF", exportPdf)}>Export PDF…</button>
                  <button onClick={() => runExport("EPUB", exportEpub)}>Export EPUB…</button>
                </div>
              </>
            )}
          </div>
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

      {settingsOpen && <Settings onClose={() => setSettingsOpen(false)} />}
      {notice && (
        <div className="toast" onClick={() => setNotice(null)}>
          {notice}
        </div>
      )}
    </div>
  );
}

export default App;
