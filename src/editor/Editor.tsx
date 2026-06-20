import { useEffect } from "react";
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
  const editor = useEditor(
    {
      extensions: editorExtensions,
      content,
      autofocus: "start",
      editorProps: { attributes: { class: "prose" } },
      onUpdate: ({ editor }) => onChange(editor.getJSON()),
    },
    [chapterId]
  );

  useEffect(() => {
    onReady(editor);
  }, [editor, onReady]);

  return <EditorContent editor={editor} className="editor-host" />;
}
