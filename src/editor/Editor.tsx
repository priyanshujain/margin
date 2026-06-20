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

export function Editor({ chapterId, content, onChange, onReady }: EditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

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
    if (editor) editor.commands.setContent(content, { emitUpdate: false });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chapterId, editor]);

  return <EditorContent editor={editor} className="editor-host" />;
}
