import { useCallback, useEffect, useState } from "react";
import type { Editor as TiptapEditor } from "@tiptap/react";
import { Sidebar } from "./Sidebar";
import { Dock } from "./Dock";
import { Icon } from "./Icon";
import { Settings } from "./Settings";
import { Editor } from "../editor/Editor";
import { FloatingToolbar } from "../editor/FloatingToolbar";
import { useBook } from "../store/useBook";
import type { Book } from "../model/book";
import { saveBook } from "../library";
import { isDesktop } from "../ipc";
import { exportEpub, exportPdf } from "../export/exporters";

const DESKTOP_ONLY =
  'Export runs in the desktop app only — open the window from "pnpm tauri dev" (you are viewing the browser preview).';

export function EditorView() {
  const book = useBook((s) => s.book);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const dirty = useBook((s) => s.dirty);
  const setChapterContent = useBook((s) => s.setChapterContent);
  const setChapterTitle = useBook((s) => s.setChapterTitle);
  const markSaved = useBook((s) => s.markSaved);

  const [editor, setEditor] = useState<TiptapEditor | null>(null);
  const [dock, setDock] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const saveNow = useCallback(() => {
    const current = useBook.getState().book;
    if (current) saveBook(current).then(markSaved).catch((e) => setNotice(`Save failed: ${e}`));
  }, [markSaved]);

  useEffect(() => {
    if (!book || !dirty || !isDesktop) return;
    const timer = setTimeout(saveNow, 800);
    return () => clearTimeout(timer);
  }, [book, dirty, saveNow]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        saveNow();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [saveNow]);

  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(null), 4000);
    return () => clearTimeout(timer);
  }, [notice]);

  const runExport = async (label: string, fn: (book: Book) => Promise<void>) => {
    setExportOpen(false);
    if (!isDesktop) {
      setNotice(DESKTOP_ONLY);
      return;
    }
    const current = useBook.getState().book;
    if (!current) return;
    try {
      await fn(current);
    } catch (e) {
      setNotice(`${label} export failed: ${e}`);
    }
  };

  if (!book) return null;
  const idx = book.chapters.findIndex((c) => c.id === activeChapterId);
  const chapter = book.chapters[idx] ?? book.chapters[0];

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <button className="doc-title" onClick={() => setSettingsOpen(true)} title="Book setup">
          {book.metadata.title || "Untitled"}
          {book.metadata.author ? ` — ${book.metadata.author}` : ""}
          {dirty && <span className="dirty-dot" />}
        </button>
        <div className="actions">
          <div className="menu-wrap">
            <button className="icon-btn" data-on={exportOpen} onClick={() => setExportOpen((v) => !v)} title="Export">
              <Icon d="M12 4v10m0 0l-4-4m4 4l4-4M5 19h14" />
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

      {settingsOpen && <Settings onClose={() => setSettingsOpen(false)} onSave={saveNow} />}
      {notice && (
        <div className="toast" onClick={() => setNotice(null)}>
          {notice}
        </div>
      )}
    </div>
  );
}
