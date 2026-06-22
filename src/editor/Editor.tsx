import { useEffect, useRef } from "react";
import { EditorContent, useEditor, type Editor as TiptapEditor } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import { editorExtensions } from "./extensions";
import { loadPosition, savePosition } from "./positions";

interface EditorProps {
  bookId: string;
  chapterId: string;
  content: JSONContent;
  onChange: (content: JSONContent) => void;
  onReady: (editor: TiptapEditor | null) => void;
}

export function Editor({ bookId, chapterId, content, onChange, onReady }: EditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const openChapter = useRef(chapterId);

  const editor = useEditor({
    extensions: editorExtensions,
    content,
    immediatelyRender: false,
    editorProps: { attributes: { class: "prose" } },
    onUpdate: ({ editor }) => onChangeRef.current(editor.getJSON()),
  });

  useEffect(() => {
    onReady(editor);
  }, [editor, onReady]);

  useEffect(() => {
    if (!editor) return;
    let active = true;
    const scroller = editor.view.dom.closest(".editor-pane") as HTMLElement | null;

    const previous = openChapter.current;
    if (previous !== chapterId) {
      const { from, to } = editor.state.selection;
      savePosition(bookId, previous, { from, to, scroll: scroller?.scrollTop ?? 0 });
    }
    openChapter.current = chapterId;

    editor.commands.setContent(content, { emitUpdate: false });

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
  }, [chapterId, editor, bookId]);

  useEffect(() => {
    if (!editor) return;
    const scroller = editor.view.dom.closest(".editor-pane") as HTMLElement | null;
    let timer: ReturnType<typeof setTimeout>;
    const persist = () => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        const { from, to } = editor.state.selection;
        savePosition(bookId, openChapter.current, { from, to, scroll: scroller?.scrollTop ?? 0 });
      }, 400);
    };
    editor.on("selectionUpdate", persist);
    scroller?.addEventListener("scroll", persist, { passive: true });
    return () => {
      clearTimeout(timer);
      editor.off("selectionUpdate", persist);
      scroller?.removeEventListener("scroll", persist);
    };
  }, [editor, bookId]);

  return <EditorContent editor={editor} className="editor-host" />;
}
