import { useEffect, useRef } from "react";
import { EditorContent, useEditor, type Editor as TiptapEditor } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import { editorExtensions } from "./extensions";
import { loadPosition, savePosition, type ChapterPosition } from "./positions";

interface EditorProps {
  bookId: string;
  chapterId: string;
  content: JSONContent;
  onChange: (content: JSONContent) => void;
  onReady: (editor: TiptapEditor | null) => void;
  onContentError: (error: Error) => void;
}

export function Editor({ bookId, chapterId, content, onChange, onReady, onContentError }: EditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const onContentErrorRef = useRef(onContentError);
  onContentErrorRef.current = onContentError;

  const latest = useRef<ChapterPosition | null>(null);

  const editor = useEditor({
    extensions: editorExtensions,
    content,
    immediatelyRender: false,
    enableContentCheck: true,
    editorProps: { attributes: { class: "prose" } },
    onContentError: ({ error }) => onContentErrorRef.current(error),
    onUpdate: ({ editor }) => onChangeRef.current(editor.getJSON()),
  });

  useEffect(() => {
    onReady(editor);
    return () => onReady(null);
  }, [editor, onReady]);

  useEffect(() => {
    if (!editor) return;
    let active = true;
    const scroller = editor.view.dom.closest(".editor-pane") as HTMLElement | null;

    const saved = loadPosition(bookId, chapterId);
    const size = editor.state.doc.content.size;
    const selection = saved ? { from: Math.min(saved.from, size), to: Math.min(saved.to, size) } : 0;
    editor.chain().setTextSelection(selection).focus(undefined, { scrollIntoView: false }).run();

    if (scroller) {
      const top = saved?.scroll ?? 0;
      const restore = () => {
        if (active) scroller.scrollTop = top;
      };
      restore();
      requestAnimationFrame(restore);
      document.fonts?.ready.then(restore).catch(() => {});
    }

    return () => {
      active = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editor]);

  useEffect(() => {
    if (!editor) return;
    const scroller = editor.view.dom.closest(".editor-pane") as HTMLElement | null;
    let timer: ReturnType<typeof setTimeout>;
    const capture = () => {
      const { from, to } = editor.state.selection;
      latest.current = { from, to, scroll: scroller?.scrollTop ?? 0 };
    };
    const persist = () => {
      capture();
      clearTimeout(timer);
      timer = setTimeout(() => savePosition(bookId, chapterId, latest.current!), 400);
    };
    editor.on("selectionUpdate", persist);
    scroller?.addEventListener("scroll", persist, { passive: true });
    return () => {
      clearTimeout(timer);
      editor.off("selectionUpdate", persist);
      scroller?.removeEventListener("scroll", persist);
      if (latest.current) savePosition(bookId, chapterId, latest.current);
    };
  }, [editor, bookId, chapterId]);

  return <EditorContent editor={editor} className="editor-host" />;
}
