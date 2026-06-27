import { useEffect, useReducer, useRef, useState, type ReactNode } from "react";
import type { Editor } from "@tiptap/react";
import { Icon } from "../components/Icon";
import type { Alignment } from "./align";

const ALIGN_ICONS: Record<Alignment, string> = {
  left: "M4 7h16M4 12h10M4 17h13",
  center: "M4 7h16M7 12h10M5 17h14",
  right: "M4 7h16M10 12h10M7 17h13",
};

function readImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

function normalizeUrl(url: string): string {
  const trimmed = url.trim();
  if (!trimmed) return "";
  if (/^(https?:\/\/|mailto:|tel:|#|\/)/i.test(trimmed)) return trimmed;
  return `https://${trimmed}`;
}

export function FloatingToolbar({ editor }: { editor: Editor | null }) {
  const fileRef = useRef<HTMLInputElement>(null);
  const linkInputRef = useRef<HTMLInputElement>(null);
  const [linkOpen, setLinkOpen] = useState(false);
  const [linkValue, setLinkValue] = useState("");
  const [alignOpen, setAlignOpen] = useState(false);
  const [, forceUpdate] = useReducer((x) => x + 1, 0);

  useEffect(() => {
    if (linkOpen) linkInputRef.current?.focus();
  }, [linkOpen]);

  useEffect(() => {
    if (!editor) return;
    const update = () => forceUpdate();
    editor.on("transaction", update);
    return () => {
      editor.off("transaction", update);
    };
  }, [editor]);

  if (!editor) return null;

  const insertImage = async (file: File) => {
    const src = await readImage(file);
    editor.chain().focus().insertContent({ type: "figure", attrs: { src, placement: "full-width" } }).run();
  };

  const openLink = () => {
    setLinkValue((editor.getAttributes("link").href as string) ?? "");
    setLinkOpen(true);
  };

  const applyLink = () => {
    const href = normalizeUrl(linkValue);
    if (!href) {
      editor.chain().focus().extendMarkRange("link").unsetLink().run();
    } else if (editor.state.selection.empty && !editor.isActive("link")) {
      editor.chain().focus().insertContent({ type: "text", text: href, marks: [{ type: "link", attrs: { href } }] }).run();
    } else {
      editor.chain().focus().extendMarkRange("link").setLink({ href }).run();
    }
    setLinkOpen(false);
  };

  const removeLink = () => {
    editor.chain().focus().extendMarkRange("link").unsetLink().run();
    setLinkOpen(false);
  };

  const align: Alignment = (editor.getAttributes("heading").align ?? editor.getAttributes("paragraph").align) || "left";

  const setAlign = (value: Alignment) => {
    editor.chain().focus().setTextAlign(value).run();
    setAlignOpen(false);
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
      <span className="tool-wrap">
        {tool(alignOpen || align !== "left", () => setAlignOpen((v) => !v), "Align", <Icon d={ALIGN_ICONS[align]} />)}
        {alignOpen && (
          <>
            <div className="link-pop-backdrop" onMouseDown={() => setAlignOpen(false)} />
            <div className="align-pop" onMouseDown={(e) => e.stopPropagation()}>
              {(["left", "center", "right"] as Alignment[]).map((value) => (
                <button
                  key={value}
                  className="tool"
                  data-on={align === value}
                  title={`Align ${value}`}
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => setAlign(value)}
                >
                  <Icon d={ALIGN_ICONS[value]} />
                </button>
              ))}
            </div>
          </>
        )}
      </span>
      <span className="tool-sep" />
      <span className="tool-wrap">
        {tool(editor.isActive("link") || linkOpen, () => (linkOpen ? setLinkOpen(false) : openLink()), "Link", <Icon d="M10 13a5 5 0 0 0 7 0l2-2a5 5 0 0 0-7-7l-1 1M14 11a5 5 0 0 0-7 0l-2 2a5 5 0 0 0 7 7l1-1" />)}
        {linkOpen && (
          <>
            <div className="link-pop-backdrop" onMouseDown={() => setLinkOpen(false)} />
            <div className="link-pop" onMouseDown={(e) => e.stopPropagation()}>
              <input
                ref={linkInputRef}
                className="link-input"
                value={linkValue}
                placeholder="https://…"
                spellCheck={false}
                onChange={(e) => setLinkValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    applyLink();
                  } else if (e.key === "Escape") {
                    e.preventDefault();
                    setLinkOpen(false);
                  }
                }}
              />
              <button className="link-btn" onMouseDown={(e) => e.preventDefault()} onClick={applyLink}>
                Apply
              </button>
              {editor.isActive("link") && (
                <button className="link-btn ghost" onMouseDown={(e) => e.preventDefault()} onClick={removeLink} title="Remove link">
                  <Icon d="M18 6L6 18M6 6l12 12" size={14} />
                </button>
              )}
            </div>
          </>
        )}
      </span>
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
