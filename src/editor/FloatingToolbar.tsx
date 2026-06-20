import { useRef, type ReactNode } from "react";
import type { Editor } from "@tiptap/react";
import { Icon } from "../components/Icon";

function readImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

export function FloatingToolbar({ editor }: { editor: Editor | null }) {
  const fileRef = useRef<HTMLInputElement>(null);
  if (!editor) return null;

  const insertImage = async (file: File) => {
    const src = await readImage(file);
    editor.chain().focus().insertContent({ type: "figure", attrs: { src, placement: "full-width" } }).run();
  };

  const tool = (active: boolean, onClick: () => void, title: string, content: ReactNode) => (
    <button
      className="tool"
      data-on={active}
      title={title}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
    >
      {content}
    </button>
  );

  return (
    <div className="editor-toolbar">
      {tool(editor.isActive("bold"), () => editor.chain().focus().toggleBold().run(), "Bold", <b>B</b>)}
      {tool(editor.isActive("italic"), () => editor.chain().focus().toggleItalic().run(), "Italic", <i>I</i>)}
      <span className="tool-sep" />
      {tool(editor.isActive("heading", { level: 2 }), () => editor.chain().focus().toggleHeading({ level: 2 }).run(), "Heading", <Icon d="M5 5v14M5 12h8M13 5v14" />)}
      {tool(editor.isActive("blockquote"), () => editor.chain().focus().toggleBlockquote().run(), "Quote", <Icon d="M7 8h4v4a4 4 0 0 1-4 4M14 8h4v4a4 4 0 0 1-4 4" />)}
      {tool(editor.isActive("bulletList"), () => editor.chain().focus().toggleBulletList().run(), "Bulleted list", <Icon d="M8 6h12M8 12h12M8 18h12M3.5 6h.01M3.5 12h.01M3.5 18h.01" />)}
      <span className="tool-sep" />
      {tool(false, () => editor.chain().focus().setHorizontalRule().run(), "Scene break", <Icon d="M5 12h5M14 12h5" />)}
      {tool(false, () => fileRef.current?.click(), "Insert image", <Icon d="M4 5h16v14H4zM4 16l4.5-4.5 3 3L16 10l4 4" />)}
      <input
        ref={fileRef}
        type="file"
        accept="image/*"
        hidden
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (file) insertImage(file);
          e.currentTarget.value = "";
        }}
      />
    </div>
  );
}
