import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Editor as TiptapEditor } from "@tiptap/react";
import * as icons from "@margin/core/icons";
import { Sidebar } from "./Sidebar";
import { Dock } from "./Dock";
import { ResizeHandle } from "./ResizeHandle";
import { Icon } from "@margin/core/ui";
import { Menu } from "./Menu";
import { ProjectSetup } from "./ProjectSetup";
import { BackupButton } from "./BackupButton";
import { CoverView } from "./CoverView";
import { FindBar } from "./FindBar";
import { Editor } from "../editor/Editor";
import { FloatingToolbar } from "../editor/FloatingToolbar";
import { COVER_ID, useBook } from "../store/useBook";
import { useBackup } from "../store/useBackup";
import { useExportPreview } from "../store/useExportPreview";
import { useProofing } from "../store/useProofing";
import { registerCommands } from "@margin/core/keys";
import { registerBookBindings } from "../keys/bindings";
import { useSettingsScreen } from "../store/useSettingsScreen";
import { useTheme } from "../store/useTheme";
import { useWidth } from "../store/useWidth";
import { WIDTH_OPTIONS } from "../width";
import { bodyNumber, chapterKind, partNumber, partRoman } from "../model/book";
import { saveBook } from "../library";
import { useEscapeLayer } from "@margin/core/hooks";
import { focusTrapped } from "../focus";
import { isDesktop, live, runWritingTool } from "../ipc";
import { useCompact } from "@margin/core/hooks";
import { runExport } from "../export/run";

export function EditorView() {
  const book = useBook((s) => s.book);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const dirty = useBook((s) => s.dirty);
  const exporting = useBook((s) => s.exporting);
  const setNotice = useBook((s) => s.setNotice);
  const setChapterContent = useBook((s) => s.setChapterContent);
  const setChapterTitle = useBook((s) => s.setChapterTitle);
  const setChapterNoTitle = useBook((s) => s.setChapterNoTitle);
  const pendingTitleFocus = useBook((s) => s.pendingTitleFocus);
  const clearTitleFocus = useBook((s) => s.clearTitleFocus);
  const markSaved = useBook((s) => s.markSaved);
  const theme = useTheme((s) => s.theme);
  const toggleTheme = useTheme((s) => s.toggle);
  const width = useWidth((s) => s.width);
  const setWidth = useWidth((s) => s.setWidth);

  const isCompact = useCompact();
  const [editor, setEditor] = useState<TiptapEditor | null>(null);
  const editorRef = useRef<TiptapEditor | null>(null);
  editorRef.current = editor;
  const lastChapterRef = useRef<string>("");
  const [sidebarOpen, setSidebarOpen] = useState(!isCompact);
  const [dock, setDock] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const [widthOpen, setWidthOpen] = useState(false);
  const [findOpen, setFindOpen] = useState(false);
  const [findExpanded, setFindExpanded] = useState(false);
  const [escArmed, setEscArmed] = useState(false);
  const escArmedRef = useRef(false);
  escArmedRef.current = escArmed;
  const escTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const spelling = useProofing((s) => s.enabled);
  const grammar = useProofing((s) => s.grammar);
  const toggleSpelling = useProofing((s) => s.toggle);
  const toggleGrammar = useProofing((s) => s.toggleGrammar);

  const coverActive = activeChapterId === COVER_ID;
  const proofingAvailable = live() && book?.metadata.language === "en";

  const toggleSidebar = () =>
    setSidebarOpen((open) => {
      if (!open && isCompact) setDock(false);
      return !open;
    });
  const toggleDock = () =>
    setDock((open) => {
      if (!open && isCompact) setSidebarOpen(false);
      return !open;
    });
  const closeDrawers = () => {
    setSidebarOpen(false);
    setDock(false);
  };

  useEscapeLayer(true, () => {
    clearTimeout(escTimer.current);
    if (escArmedRef.current) {
      setEscArmed(false);
      useBook.getState().closeBook();
      return;
    }
    (document.activeElement as HTMLElement | null)?.blur();
    setEscArmed(true);
    escTimer.current = setTimeout(() => setEscArmed(false), 1500);
  });
  useEscapeLayer(isCompact && (sidebarOpen || dock), closeDrawers);
  useEscapeLayer(widthOpen, () => setWidthOpen(false));
  useEscapeLayer(exportOpen, () => setExportOpen(false));
  useEscapeLayer(moreOpen, () => setMoreOpen(false));

  useEffect(() => () => clearTimeout(escTimer.current), []);

  const openFind = useCallback((expanded: boolean) => {
    setFindExpanded(expanded);
    setFindOpen(true);
  }, []);

  const focusEditorSoon = useCallback(() => {
    if (useBook.getState().pendingTitleFocus || focusTrapped()) return;
    requestAnimationFrame(() => editorRef.current?.commands.focus(undefined, { scrollIntoView: false }));
  }, []);

  const switchChapter = useCallback(
    (dir: 1 | -1) => {
      useBook.getState().goToAdjacentChapter(dir);
      focusEditorSoon();
    },
    [focusEditorSoon]
  );


  const saveNow = useCallback(() => {
    const current = useBook.getState().book;
    if (!current) return;
    saveBook(current)
      .then(() => {
        if (useBook.getState().book === current) markSaved();
        useBackup.getState().refresh();
      })
      .catch((e) => setNotice(`Save failed: ${e}`));
  }, [markSaved, setNotice]);

  useEffect(() => {
    if (!book || !dirty || !live()) return;
    const timer = setTimeout(saveNow, 800);
    return () => clearTimeout(timer);
  }, [book, dirty, saveNow]);

  const commands = useRef<Record<string, () => void>>({});
  commands.current = {
    save: saveNow,
    "export-pdf": () => handleExport("pdf"),
    "export-epub": () => handleExport("epub"),
    "project-setup": () => setSettingsOpen(true),
    "all-projects": () => useBook.getState().closeBook(),
    find: () => openFind(false),
    "find-replace": () => openFind(true),
    "toggle-chapters": toggleSidebar,
    "toggle-preview": toggleDock,
    "next-chapter": () => switchChapter(1),
    "prev-chapter": () => switchChapter(-1),
    "toggle-spelling": toggleSpelling,
    "toggle-grammar": toggleGrammar,
    "writing-proofread": () => void runWritingTool("Proofread").catch(() => {}),
    "writing-rewrite": () => void runWritingTool("Rewrite").catch(() => {}),
  };

  useEffect(() => {
    const unbind = registerBookBindings();
    const unregister = registerCommands(
      Object.fromEntries(
        Object.keys(commands.current).map((id) => [
          id,
          () => {
            if (!focusTrapped() && !useSettingsScreen.getState().open) commands.current[id]();
          },
        ]),
      ),
    );
    return () => {
      unregister();
      unbind();
    };
  }, []);

  useEffect(() => {
    setSidebarOpen(!isCompact);
    if (isCompact) setDock(false);
  }, [isCompact]);

  function handleExport(format: "pdf" | "epub") {
    setExportOpen(false);
    if (format === "pdf" && isDesktop) useExportPreview.getState().openPreview();
    else runExport(format);
  }

  if (!book) return null;
  const activeIdx = book.chapters.findIndex((c) => c.id === activeChapterId);
  const activeChapter = book.chapters[activeIdx];
  if (activeChapter) lastChapterRef.current = activeChapter.id;
  const editorIdx = activeChapter ? activeIdx : book.chapters.findIndex((c) => c.id === lastChapterRef.current);
  const chapter = book.chapters[editorIdx] ?? book.chapters[0];
  const realIdx = book.chapters.findIndex((c) => c.id === chapter?.id);
  const kind = chapter ? chapterKind(chapter) : "body";
  const eyebrow =
    kind === "body"
      ? `Chapter ${bodyNumber(book.chapters, realIdx) ?? ""}`
      : kind === "part"
        ? `Part ${partRoman(partNumber(book.chapters, realIdx) ?? 0)}`
        : kind === "front"
          ? "Front matter"
          : "Back matter";

  return (
    <div className="app" data-compact={isCompact} data-sidebar={sidebarOpen} data-dock={dock}>
      <header className="titlebar" data-tauri-drag-region>
        <div className="lead">
          <button className="icon-btn" data-on={sidebarOpen} onClick={toggleSidebar} title="Toggle chapters" aria-label="Toggle chapters">
            <Icon d={icons.SIDEBAR} />
          </button>
        </div>
        <button className="doc-title" onClick={() => setSettingsOpen(true)} title="Project setup">
          {book.metadata.title || "Untitled"}
          {dirty && <span className="dirty-dot" />}
        </button>
        <div className="actions">
          {live() && !isCompact && <BackupButton />}
          <button className="icon-btn" data-on={findOpen} onClick={() => (findOpen ? setFindOpen(false) : openFind(false))} title="Find (⌘F)">
            <Icon d={icons.SEARCH} />
          </button>
          {!isCompact && proofingAvailable && !coverActive && (
            <>
              <button className="icon-btn" data-on={spelling} onClick={toggleSpelling} title="Check spelling">
                <Icon d={icons.SPELLING} />
              </button>
              <button className="icon-btn" data-on={grammar} onClick={toggleGrammar} title="Check grammar">
                <Icon d={icons.GRAMMAR} />
              </button>
            </>
          )}
          {!isCompact && (
            <div className="menu-wrap">
              <button className="icon-btn" data-on={widthOpen} onClick={() => setWidthOpen((v) => !v)} title="Editor width">
                <Icon d={icons.WIDTH} />
              </button>
              <Menu open={widthOpen} onClose={() => setWidthOpen(false)}>
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
              </Menu>
            </div>
          )}
          {!isCompact && (
            <div className="menu-wrap">
              <button className="icon-btn" data-on={exportOpen} onClick={() => setExportOpen((v) => !v)} title="Export">
                <Icon d={icons.EXPORT} />
              </button>
              <Menu open={exportOpen} onClose={() => setExportOpen(false)}>
                <button onClick={() => handleExport("pdf")}>Export PDF…</button>
                <button onClick={() => handleExport("epub")}>Export EPUB…</button>
              </Menu>
            </div>
          )}
          {!isCompact && (
            <button className="icon-btn" onClick={toggleTheme} title={theme === "dark" ? "Light mode" : "Dark mode"}>
              {theme === "dark" ? (
                <Icon>
                  <circle cx={icons.SUN_DISC.cx} cy={icons.SUN_DISC.cy} r={icons.SUN_DISC.r} />
                  <path d={icons.SUN_RAYS} />
                </Icon>
              ) : (
                <Icon d={icons.MOON} />
              )}
            </button>
          )}
          <button className="icon-btn" data-on={dock} onClick={toggleDock} title="Toggle preview">
            <Icon d={icons.DOCK} />
          </button>
          {isCompact && (
            <div className="menu-wrap">
              <button className="icon-btn" data-on={moreOpen} onClick={() => setMoreOpen((v) => !v)} title="More" aria-label="More options">
                <Icon d={icons.MORE} />
              </button>
              <Menu open={moreOpen} onClose={() => setMoreOpen(false)}>
                <div className="menu-label">Editor width</div>
                {WIDTH_OPTIONS.map((w) => (
                  <button
                    key={w.id}
                    data-on={width === w.id}
                    onClick={() => {
                      setWidth(w.id);
                      setMoreOpen(false);
                    }}
                  >
                    {w.label}
                  </button>
                ))}
                <div className="menu-sep" />
                <button
                  onClick={() => {
                    setMoreOpen(false);
                    handleExport("pdf");
                  }}
                >
                  Export PDF…
                </button>
                <button
                  onClick={() => {
                    setMoreOpen(false);
                    handleExport("epub");
                  }}
                >
                  Export EPUB…
                </button>
                {proofingAvailable && !coverActive && (
                  <>
                    <div className="menu-sep" />
                    <button data-on={spelling} onClick={toggleSpelling}>
                      Check spelling
                    </button>
                    <button data-on={grammar} onClick={toggleGrammar}>
                      Check grammar
                    </button>
                  </>
                )}
                <div className="menu-sep" />
                <button onClick={toggleTheme}>{theme === "dark" ? "Light mode" : "Dark mode"}</button>
              </Menu>
            </div>
          )}
        </div>
      </header>

      <FindBar
        editor={coverActive ? null : editor}
        open={findOpen && !coverActive}
        initialExpanded={findExpanded}
        onClose={() => setFindOpen(false)}
      />

      <div className="body">
        <Sidebar
          onNavigate={() => {
            if (isCompact) setSidebarOpen(false);
            focusEditorSoon();
          }}
        />
        {sidebarOpen && !isCompact && <ResizeHandle pane="sidebar" />}
        <main className="editor-pane">
          {coverActive && <CoverView />}
          <article className="sheet" hidden={coverActive}>
            <header className="chapter-opener">
              <div className="chapter-num">{eyebrow}</div>
              <ChapterTitleInput
                value={chapter.title}
                placeholder={
                  chapter.noTitle ? "No title" : kind === "body" ? "Chapter title" : kind === "part" ? "Part title (optional)" : "Page title"
                }
                disabled={!!chapter.noTitle}
                focusRequest={pendingTitleFocus === chapter.id}
                onFocused={clearTitleFocus}
                onChange={(value) => setChapterTitle(chapter.id, value)}
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
              onContentError={() =>
                setNotice("This chapter contains content Margin couldn't fully read; editing may drop the unrecognized parts.")
              }
            />
          </article>
          {!coverActive && <FloatingToolbar editor={editor} />}
        </main>
        {dock && !isCompact && <ResizeHandle pane="dock" />}
        {dock && <Dock />}
        {isCompact && (sidebarOpen || dock) && (
          <button className="drawer-scrim" aria-label="Close panel" onClick={closeDrawers} />
        )}
      </div>

      {escArmed && (
        <div className="esc-hint">
          Press <kbd>Esc</kbd> again for All projects
        </div>
      )}

      {settingsOpen && <ProjectSetup onClose={() => setSettingsOpen(false)} onSave={saveNow} />}
      {exporting && (
        <div className="export-overlay">
          <div className="spinner" />
          <p>Exporting {exporting}…</p>
        </div>
      )}
    </div>
  );
}

function ChapterTitleInput({
  value,
  placeholder,
  disabled,
  focusRequest,
  onFocused,
  onChange,
}: {
  value: string;
  placeholder: string;
  disabled: boolean;
  focusRequest: boolean;
  onFocused: () => void;
  onChange: (value: string) => void;
}) {
  const ref = useRef<HTMLTextAreaElement>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [value, disabled]);

  useEffect(() => {
    if (!focusRequest || disabled) return;
    const el = ref.current;
    if (!el) return;
    el.focus();
    el.select();
    onFocused();
  }, [focusRequest, disabled, onFocused]);

  return (
    <textarea
      ref={ref}
      className="chapter-title-input"
      rows={1}
      value={value}
      placeholder={placeholder}
      spellCheck={false}
      disabled={disabled}
      onKeyDown={(e) => e.key === "Enter" && e.preventDefault()}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}
