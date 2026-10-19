import { Extension } from "@tiptap/core";
import type { Editor, JSONContent } from "@tiptap/core";
import { Plugin } from "@tiptap/pm/state";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { isDesktop } from "../ipc";
import { useBook } from "../store/useBook";

function readImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

function imageFiles(data: DataTransfer | null): File[] {
  if (!data) return [];
  const files = Array.from(data.files).filter((f) => f.type.startsWith("image/"));
  if (files.length) return files;
  return Array.from(data.items)
    .filter((item) => item.kind === "file" && item.type.startsWith("image/"))
    .map((item) => item.getAsFile())
    .filter((f): f is File => f !== null);
}

function plainContent(text: string): JSONContent[] {
  return text
    .replace(/\r\n?/g, "\n")
    .split(/\n{2,}/)
    .map((block) => {
      const content: JSONContent[] = [];
      block.split("\n").forEach((line, i) => {
        if (i > 0) content.push({ type: "hardBreak" });
        if (line) content.push({ type: "text", text: line });
      });
      return content.length ? { type: "paragraph", content } : { type: "paragraph" };
    });
}

async function pastePlain(editor: Editor): Promise<void> {
  try {
    const text = await (isDesktop ? readText() : navigator.clipboard.readText());
    if (text && !editor.isDestroyed) {
      editor.chain().focus(undefined, { scrollIntoView: false }).insertContent(plainContent(text)).run();
    }
  } catch (error) {
    useBook.getState().setNotice(`Could not paste without formatting: ${error}`);
  }
}

export const Paste = Extension.create({
  name: "pasteHandler",

  addKeyboardShortcuts() {
    const editor = this.editor;
    return {
      "Mod-Shift-v": () => {
        void pastePlain(editor);
        return true;
      },
    };
  },

  addProseMirrorPlugins() {
    const editor = this.editor;
    return [
      new Plugin({
        props: {
          handlePaste(_view, event) {
            const files = imageFiles(event.clipboardData);
            if (!files.length) return false;
            event.preventDefault();
            files.forEach((file) =>
              readImage(file).then((src) =>
                editor.chain().focus().insertContent({ type: "figure", attrs: { src, placement: "full-width" } }).run(),
              ),
            );
            return true;
          },
        },
      }),
    ];
  },
});
