import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { EditorContent, type Editor as TiptapEditor } from "@tiptap/react";
import { EditorState } from "@tiptap/pm/state";
import type { JSONContent } from "@tiptap/core";
import { loadPosition, savePosition, type ChapterPosition } from "./positions";
import { loadChapterState, saveChapterState, sharedEditor, type ChapterState } from "./session";
import { useBook } from "../store/useBook";

interface EditorProps {
  bookId: string;
  chapterId: string;
  content: JSONContent;
  onChange: (content: JSONContent) => void;
  onReady: (editor: TiptapEditor | null) => void;
  onContentError: (error: Error) => void;
}

function buildState(editor: TiptapEditor, content: JSONContent, onError: (error: Error) => void): EditorState {
  const base = editor.view.state;
  try {
    const doc = editor.schema.nodeFromJSON(content);
    doc.check();
    return EditorState.create({ doc, plugins: base.plugins });
  } catch (error) {
    onError(error as Error);
    return EditorState.create({ schema: base.schema, plugins: base.plugins });
  }
}

function stillMatches(editor: TiptapEditor, entry: ChapterState, content: JSONContent): boolean {
  if (entry.state.schema !== editor.schema) return false;
  if (entry.content === content) return true;
  try {
    return entry.state.doc.eq(editor.schema.nodeFromJSON(content));
  } catch {
    return false;
  }
}

export function Editor({ bookId, chapterId, content, onChange, onReady, onContentError }: EditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const onContentErrorRef = useRef(onContentError);
  onContentErrorRef.current = onContentError;

  const [editor] = useState(() =>
    sharedEditor({
      onUpdate: (next) => onChangeRef.current(next),
      onContentError: (error) => onContentErrorRef.current(error),
    })
  );

  const active = useRef({ bookId: "", chapterId: "" });
  const latest = useRef<ChapterPosition | null>(null);
  const restoreToken = useRef(0);

  const scrollerOf = (ed: TiptapEditor) => ed.view.dom.closest(".editor-pane") as HTMLElement | null;

  const restorePosition = (ed: TiptapEditor, id: string, focus: boolean) => {
    const scroller = scrollerOf(ed);
    const saved = loadPosition(bookId, id);
    const size = ed.state.doc.content.size;
    const selection = saved ? { from: Math.min(saved.from, size), to: Math.min(saved.to, size) } : 0;
    const chain = ed.chain().setTextSelection(selection);
    if (focus) chain.focus(undefined, { scrollIntoView: false });
    chain.run();
    if (scroller) {
      const token = ++restoreToken.current;
      const top = saved?.scroll ?? 0;
      const apply = () => {
        if (restoreToken.current === token) scroller.scrollTop = top;
      };
      apply();
      requestAnimationFrame(apply);
      document.fonts?.ready.then(apply).catch(() => {});
    }
  };

  const stash = (ed: TiptapEditor) => {
    const { bookId: prevBook, chapterId: prevChapter } = active.current;
    if (!prevChapter) return;
    const scroller = scrollerOf(ed);
    const { from, to } = ed.state.selection;
    const scroll = scroller?.scrollTop ?? loadChapterState(prevBook, prevChapter)?.scroll ?? 0;
    const stored = useBook.getState().book?.chapters.find((c) => c.id === prevChapter)?.content ?? ed.getJSON();
    saveChapterState(prevBook, prevChapter, { state: ed.view.state, content: stored, scroll });
    savePosition(prevBook, prevChapter, { from, to, scroll });
  };

  const applyChapter = (ed: TiptapEditor, id: string, next: JSONContent, focus: boolean) => {
    const entry = loadChapterState(bookId, id);
    if (entry && stillMatches(ed, entry, next)) {
      entry.content = next;
      ed.view.updateState(entry.state);
      const scroller = scrollerOf(ed);
      if (scroller) scroller.scrollTop = entry.scroll;
      ++restoreToken.current;
      if (focus) ed.commands.focus(undefined, { scrollIntoView: false });
      return;
    }
    ed.view.updateState(buildState(ed, next, onContentErrorRef.current));
    saveChapterState(bookId, id, { state: ed.view.state, content: next, scroll: 0 });
    restorePosition(ed, id, focus);
  };

  useLayoutEffect(() => {
    const prev = active.current;
    if (prev.bookId === bookId && prev.chapterId === chapterId) return;
    stash(editor);
    applyChapter(editor, chapterId, content, !prev.chapterId);
    active.current = { bookId, chapterId };
    latest.current = null;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bookId, chapterId, content, editor]);

  useEffect(() => {
    onReady(editor);
    return () => {
      stash(editor);
      onReady(null);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editor]);

  useEffect(() => {
    const scroller = scrollerOf(editor);
    let timer: ReturnType<typeof setTimeout>;
    const capture = () => {
      const { from, to } = editor.state.selection;
      const scroll = scroller?.scrollTop ?? 0;
      latest.current = { from, to, scroll };
      const entry = loadChapterState(active.current.bookId, active.current.chapterId);
      if (entry) entry.scroll = scroll;
    };
    const persist = () => {
      capture();
      clearTimeout(timer);
      timer = setTimeout(() => savePosition(active.current.bookId, active.current.chapterId, latest.current!), 400);
    };
    editor.on("selectionUpdate", persist);
    scroller?.addEventListener("scroll", persist, { passive: true });
    return () => {
      clearTimeout(timer);
      editor.off("selectionUpdate", persist);
      scroller?.removeEventListener("scroll", persist);
      if (latest.current) savePosition(active.current.bookId, active.current.chapterId, latest.current);
    };
  }, [editor]);

  return <EditorContent editor={editor} className="editor-host" />;
}
