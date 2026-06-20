import type { Extensions } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";

export const editorExtensions: Extensions = [
  StarterKit.configure({
    heading: { levels: [2, 3] },
    codeBlock: false,
  }),
  Placeholder.configure({ placeholder: "Begin your chapter…" }),
];
