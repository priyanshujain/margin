import type { Extensions } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import { Figure } from "./figure";

export const editorExtensions: Extensions = [
  StarterKit.configure({
    heading: { levels: [2, 3] },
    codeBlock: false,
  }),
  Placeholder.configure({
    placeholder: ({ node }) => (node.type.name === "heading" ? "" : "Begin your chapter…"),
  }),
  Figure,
];
