import { useEffect, useLayoutEffect, useRef } from "react";
import { EditorContent, useEditor, type Editor as TiptapEditor } from "@tiptap/react";
import { EditorState } from "@tiptap/pm/state";
import type { JSONContent } from "@tiptap/core";
import { editorExtensions } from "./extensions";
import { loadPosition, savePosition, type ChapterPosition } from "./positions";
import { useBook } from "../store/useBook";

interface EditorProps {
  bookId: string;
  chapterId: string;
  content: JSONContent;
  onChange: (content: JSONContent) => void;
  onReady: (editor: TiptapEditor | null) => void;
  onContentError: (error: Error) => void;
}

interface Cached {
  state: EditorState;
  content: JSONContent;
  scroll: number;
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

export function Editor({ bookId, chapterId, content, onChange, onReady, onContentError }: EditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const onContentErrorRef = useRef(onContentError);
  onContentErrorRef.current = onContentError;

  const cache = useRef(new Map<string, Cached>());
  const activeId = useRef(chapterId);
  const latest = useRef<ChapterPosition | null>(null);
  const restoreToken = useRef(0);

  const editor = useEditor({
    extensions: editorExtensions,
    content,
    immediatelyRender: false,
    enableContentCheck: true,
    editorProps: { attributes: { class: "prose" } },
    onContentError: ({ error }) => onContentErrorRef.current(error),
    onUpdate: ({ editor }) => onChangeRef.current(editor.getJSON()),
  });

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

  const stash = (ed: TiptapEditor, id: string) => {
    const scroller = scrollerOf(ed);
    const { from, to } = ed.state.selection;
    const scroll = scroller?.scrollTop ?? 0;
    const stored = useBook.getState().book?.chapters.find((c) => c.id === id)?.content ?? cache.current.get(id)?.content ?? ed.getJSON();
    cache.current.set(id, { state: ed.view.state, content: stored, scroll });
    savePosition(bookId, id, { from, to, scroll });
  };

  useEffect(() => {
    if (!editor) return;
    cache.current.set(chapterId, { state: editor.view.state, content, scroll: 0 });
    activeId.current = chapterId;
    restorePosition(editor, chapterId, true);
    onReady(editor);
    return () => onReady(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editor]);

  useLayoutEffect(() => {
    if (!editor) return;
    const prev = activeId.current;
    if (prev === chapterId) return;

    stash(editor, prev);
    const entry = cache.current.get(chapterId);
    if (entry && entry.content === content) {
      editor.view.updateState(entry.state);
      const scroller = scrollerOf(editor);
      if (scroller) scroller.scrollTop = entry.scroll;
      ++restoreToken.current;
    } else {
      editor.view.updateState(buildState(editor, content, onContentErrorRef.current));
      cache.current.set(chapterId, { state: editor.view.state, content, scroll: 0 });
      restorePosition(editor, chapterId, false);
    }
    activeId.current = chapterId;
    latest.current = null;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chapterId, content, editor]);

  useEffect(() => {
    if (!editor) return;
    const scroller = scrollerOf(editor);
    let timer: ReturnType<typeof setTimeout>;
    const capture = () => {
      const { from, to } = editor.state.selection;
      const scroll = scroller?.scrollTop ?? 0;
      latest.current = { from, to, scroll };
      const entry = cache.current.get(activeId.current);
      if (entry) entry.scroll = scroll;
    };
    const persist = () => {
      capture();
      clearTimeout(timer);
      timer = setTimeout(() => savePosition(bookId, activeId.current, latest.current!), 400);
    };
    editor.on("selectionUpdate", persist);
    scroller?.addEventListener("scroll", persist, { passive: true });
    return () => {
      clearTimeout(timer);
      editor.off("selectionUpdate", persist);
      scroller?.removeEventListener("scroll", persist);
      if (latest.current) savePosition(bookId, activeId.current, latest.current);
    };
  }, [editor, bookId]);

  return <EditorContent editor={editor} className="editor-host" />;
}
