import { useEffect, useRef } from "react";
import { EditorContent, useEditor, type Editor as TiptapEditor } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import { editorExtensions } from "./extensions";

interface EditorProps {
  chapterId: string;
  content: JSONContent;
  onChange: (content: JSONContent) => void;
  onReady: (editor: TiptapEditor | null) => void;
}

interface ChapterPosition {
  from: number;
  to: number;
  scroll: number;
}

export function Editor({ chapterId, content, onChange, onReady }: EditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const positions = useRef(new Map<string, ChapterPosition>());
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
    const scroller = editor.view.dom.closest(".editor-pane") as HTMLElement | null;

    const previous = openChapter.current;
    const switching = previous !== chapterId;
    if (switching) {
      const { from, to } = editor.state.selection;
      positions.current.set(previous, { from, to, scroll: scroller?.scrollTop ?? 0 });
    }
    openChapter.current = chapterId;

    editor.commands.setContent(content, { emitUpdate: false });

    const saved = positions.current.get(chapterId);
    const size = editor.state.doc.content.size;
    const selection = saved ? { from: Math.min(saved.from, size), to: Math.min(saved.to, size) } : 0;
    if (switching) {
      editor.chain().setTextSelection(selection).focus(undefined, { scrollIntoView: false }).run();
    } else {
      editor.commands.setTextSelection(selection);
    }

    if (scroller) {
      const top = saved?.scroll ?? 0;
      scroller.scrollTop = top;
      requestAnimationFrame(() => {
        scroller.scrollTop = top;
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chapterId, editor]);

  return <EditorContent editor={editor} className="editor-host" />;
}
