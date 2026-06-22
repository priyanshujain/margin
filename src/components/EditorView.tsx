import { useCallback, useEffect, useState } from "react";
import type { Editor as TiptapEditor } from "@tiptap/react";
import { Sidebar } from "./Sidebar";
import { Dock } from "./Dock";
import { ResizeHandle } from "./ResizeHandle";
import { Icon } from "./Icon";
import { Settings } from "./Settings";
import { CoverView } from "./CoverView";
import { FindBar } from "./FindBar";
import { ProofPopover } from "./ProofPopover";
import { Editor } from "../editor/Editor";
import { FloatingToolbar } from "../editor/FloatingToolbar";
import type { ProofCoords, ProofIssue, ProofingStorage } from "../editor/proofing";
import { COVER_ID, useBook } from "../store/useBook";
import { useProofing } from "../store/useProofing";
import { useTheme } from "../store/useTheme";
import { useWidth } from "../store/useWidth";
import { WIDTH_OPTIONS } from "../width";
import { bodyNumber, chapterKind } from "../model/book";
import { saveBook } from "../library";
import { isDesktop } from "../ipc";
import { issueSignature, rememberWord, runProof } from "../proofing";
import { runExport } from "../export/run";

export function EditorView() {
  const book = useBook((s) => s.book);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const dirty = useBook((s) => s.dirty);
  const exporting = useBook((s) => s.exporting);
  const notice = useBook((s) => s.notice);
  const setNotice = useBook((s) => s.setNotice);
  const setChapterContent = useBook((s) => s.setChapterContent);
  const setChapterTitle = useBook((s) => s.setChapterTitle);
  const setChapterNoTitle = useBook((s) => s.setChapterNoTitle);
  const markSaved = useBook((s) => s.markSaved);
  const theme = useTheme((s) => s.theme);
  const toggleTheme = useTheme((s) => s.toggle);
  const width = useWidth((s) => s.width);
  const setWidth = useWidth((s) => s.setWidth);

  const [editor, setEditor] = useState<TiptapEditor | null>(null);
  const [dock, setDock] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [widthOpen, setWidthOpen] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [findExpanded, setFindExpanded] = useState(false);
  const [proofPopover, setProofPopover] = useState<{ issue: ProofIssue; coords: ProofCoords } | null>(null);

  const spelling = useProofing((s) => s.spelling);
  const grammar = useProofing((s) => s.grammar);
  const ignored = useProofing((s) => s.ignored);
  const toggleSpelling = useProofing((s) => s.toggleSpelling);
  const toggleGrammar = useProofing((s) => s.toggleGrammar);

  const coverActive = activeChapterId === COVER_ID;
  const proofingAvailable = isDesktop && book?.metadata.language === "en";

  const openFind = useCallback((expanded: boolean) => {
    setFindExpanded(expanded);
    setFindOpen(true);
  }, []);

  const reproof = useCallback(() => {
    if (!editor) return;
    const state = useProofing.getState();
    if (!proofingAvailable || (!state.spelling && !state.grammar)) {
      editor.commands.clearProofIssues();
      return;
    }
    runProof(editor.state.doc, { spelling: state.spelling, grammar: state.grammar }, state.ignored)
      .then((issues) => editor.commands.setProofIssues(issues))
      .catch(() => {});
  }, [editor, proofingAvailable]);

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
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        openFind(e.altKey);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [saveNow, openFind]);

  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(null), 4000);
    return () => clearTimeout(timer);
  }, [notice]);

  useEffect(() => {
    if (!editor) return;
    const storage = (editor.storage as unknown as Record<string, ProofingStorage>).proofing;
    storage.onClickIssue = (issue, coords) => setProofPopover({ issue, coords });
    const closePopover = () => setProofPopover(null);
    editor.on("update", closePopover);
    return () => {
      storage.onClickIssue = null;
      editor.off("update", closePopover);
    };
  }, [editor]);

  useEffect(() => {
    if (!editor) return;
    if (!proofingAvailable || coverActive || (!spelling && !grammar)) {
      editor.commands.clearProofIssues();
      return;
    }
    let timer: ReturnType<typeof setTimeout>;
    const schedule = (delay: number) => {
      clearTimeout(timer);
      timer = setTimeout(reproof, delay);
    };
    editor.commands.clearProofIssues();
    schedule(150);
    const onUpdate = () => schedule(500);
    editor.on("update", onUpdate);
    return () => {
      clearTimeout(timer);
      editor.off("update", onUpdate);
    };
  }, [editor, spelling, grammar, ignored, activeChapterId, coverActive, proofingAvailable, reproof]);

  const handleExport = (format: "pdf" | "epub") => {
    setExportOpen(false);
    runExport(format);
  };

  if (!book) return null;
  const idx = book.chapters.findIndex((c) => c.id === activeChapterId);
  const chapter = book.chapters[idx] ?? book.chapters[0];
  const realIdx = book.chapters.findIndex((c) => c.id === chapter?.id);
  const kind = chapter ? chapterKind(chapter) : "body";
  const eyebrow =
    kind === "body" ? `Chapter ${bodyNumber(book.chapters, realIdx) ?? ""}` : kind === "front" ? "Front matter" : "Back matter";

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <button className="doc-title" onClick={() => setSettingsOpen(true)} title="Book setup">
          {book.metadata.title || "Untitled"}
          {dirty && <span className="dirty-dot" />}
        </button>
        <div className="actions">
          <button className="icon-btn" data-on={findOpen} onClick={() => (findOpen ? setFindOpen(false) : openFind(false))} title="Find (⌘F)">
            <Icon d="M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4" />
          </button>
          {proofingAvailable && !coverActive && (
            <>
              <button className="icon-btn" data-on={spelling} onClick={toggleSpelling} title="Check spelling">
                <Icon d="M4 17l4-10 4 10M5.4 13.4h5.2M15 17l2.5 2.5L22 14" />
              </button>
              <button className="icon-btn" data-on={grammar} onClick={toggleGrammar} title="Check grammar">
                <Icon d="M4 7h16M4 12h12M4 17h7M13.5 18.5c1-1.2 2-1.2 3 0s2 1.2 3 0" />
              </button>
            </>
          )}
          <div className="menu-wrap">
            <button className="icon-btn" data-on={widthOpen} onClick={() => setWidthOpen((v) => !v)} title="Editor width">
              <Icon d="M3 5v14M21 5v14M7 12h10M7 12l3-3M7 12l3 3M17 12l-3-3M17 12l-3 3" />
            </button>
            {widthOpen && (
              <>
                <div className="menu-backdrop" onClick={() => setWidthOpen(false)} />
                <div className="menu">
                  {WIDTH_OPTIONS.map((w) => (
                    <button
                      key={w.id}
                      data-on={width === w.id}
                      onClick={() => {
                        setWidth(w.id);
                        setWidthOpen(false);
                      }}
                    >
                      {w.label}
                    </button>
                  ))}
                </div>
              </>
            )}
          </div>
          <div className="menu-wrap">
            <button className="icon-btn" data-on={exportOpen} onClick={() => setExportOpen((v) => !v)} title="Export">
              <Icon d="M5 13v6h14v-6M12 16V3M8 7l4-4 4 4" />
            </button>
            {exportOpen && (
              <>
                <div className="menu-backdrop" onClick={() => setExportOpen(false)} />
                <div className="menu">
                  <button onClick={() => handleExport("pdf")}>Export PDF…</button>
                  <button onClick={() => handleExport("epub")}>Export EPUB…</button>
                </div>
              </>
            )}
          </div>
          <button className="icon-btn" onClick={toggleTheme} title={theme === "dark" ? "Light mode" : "Dark mode"}>
            {theme === "dark" ? (
              <Icon>
                <circle cx="12" cy="12" r="4" />
                <path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M19.1 4.9l-1.4 1.4M6.3 17.7l-1.4 1.4" />
              </Icon>
            ) : (
              <Icon d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
            )}
          </button>
          <button className="icon-btn" data-on={dock} onClick={() => setDock(!dock)} title="Toggle preview">
            <Icon d="M3 4.5h18v15H3zM14 4.5v15" />
          </button>
        </div>
      </header>

      <FindBar
        editor={coverActive ? null : editor}
        open={findOpen && !coverActive}
        initialExpanded={findExpanded}
        onClose={() => setFindOpen(false)}
      />

      <div className="body">
        <Sidebar />
        <ResizeHandle pane="sidebar" />
        <main className="editor-pane">
          {coverActive ? (
            <CoverView />
          ) : (
            <>
              <article className="sheet">
                <header className="chapter-opener">
                  <div className="chapter-num">{eyebrow}</div>
                  <input
                    className="chapter-title-input"
                    value={chapter.noTitle ? "" : chapter.title}
                    placeholder={chapter.noTitle ? "No title" : kind === "body" ? "Chapter title" : "Page title"}
                    spellCheck={false}
                    disabled={chapter.noTitle}
                    onChange={(e) => setChapterTitle(chapter.id, e.target.value)}
                  />
                  {(chapter.noTitle || !chapter.title.trim()) && (
                    <label className="no-title-check">
                      <input
                        type="checkbox"
                        checked={!!chapter.noTitle}
                        onChange={(e) => setChapterNoTitle(chapter.id, e.target.checked)}
                      />
                      No title
                    </label>
                  )}
                </header>
                <Editor
                  bookId={book.id}
                  chapterId={chapter.id}
                  content={chapter.content}
                  onChange={(content) => setChapterContent(chapter.id, content)}
                  onReady={setEditor}
                />
              </article>
              <FloatingToolbar editor={editor} />
            </>
          )}
        </main>
        {dock && <ResizeHandle pane="dock" />}
        {dock && <Dock />}
      </div>

      {proofPopover && editor && (
        <ProofPopover
          issue={proofPopover.issue}
          coords={proofPopover.coords}
          onReplace={(suggestion) => {
            const { from, to } = proofPopover.issue;
            if (suggestion === "") editor.chain().focus().deleteRange({ from, to }).run();
            else editor.chain().focus().insertContentAt({ from, to }, suggestion).run();
            setProofPopover(null);
          }}
          onIgnore={() => {
            useProofing.getState().ignore(issueSignature(proofPopover.issue));
            setProofPopover(null);
          }}
          onRemember={() => {
            rememberWord(proofPopover.issue.word).then(reproof).catch(() => {});
            setProofPopover(null);
          }}
          onClose={() => setProofPopover(null)}
        />
      )}

      {settingsOpen && <Settings onClose={() => setSettingsOpen(false)} onSave={saveNow} />}
      {exporting && (
        <div className="export-overlay">
          <div className="spinner" />
          <p>Exporting {exporting}…</p>
        </div>
      )}
      {notice && (
        <div className="toast" onClick={() => setNotice(null)}>
          {notice}
        </div>
      )}
    </div>
  );
}
