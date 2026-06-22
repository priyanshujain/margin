import type { Extensions } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import { Figure } from "./figure";
import { ParagraphIndent } from "./indent";
import { SearchHighlight } from "./search";
import { Proofing } from "./proofing";

export const editorExtensions: Extensions = [
  StarterKit.configure({
    heading: { levels: [2, 3] },
    codeBlock: false,
    link: { openOnClick: false, autolink: true, defaultProtocol: "https" },
  }),
  Placeholder.configure({
    placeholder: ({ node }) => (node.type.name === "heading" ? "" : "Begin your chapter…"),
  }),
  Figure,
  ParagraphIndent,
  SearchHighlight,
  Proofing,
];
