import { EditorContent, useEditor } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import { editorExtensions } from "./extensions";

interface EditorProps {
  chapterId: string;
  content: JSONContent;
  onChange: (content: JSONContent) => void;
}

export function Editor({ chapterId, content, onChange }: EditorProps) {
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

  return <EditorContent editor={editor} className="editor-host" />;
}
