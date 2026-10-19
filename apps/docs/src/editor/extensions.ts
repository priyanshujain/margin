// The editor's schema is the contract's schema, node for node and mark for mark.
//
// src/model/schema.ts is frozen and the markdown bridge is written against it: it produces
// callouts, toggles, task lists, tables, math and raw blocks, marks named `strong`, `em` and
// `strikethrough`, a code block that carries the `meta` off its opening fence and lists that
// remember whether they were tight. StarterKit's own nodes are a different schema: no callout, no
// toggle, no raw, `bold` where this one says `strong`, and a code block with nowhere to keep
// `meta`. Adopting them would mean the bridge parsing a document the editor cannot hold, and every
// attribute the two disagree about is a piece of somebody's file quietly lost on the next save.
//
// So the node and mark extensions below are generated from those specs rather than restated: one
// TipTap extension per entry in the contract, carrying that entry's own content expression,
// attributes, parse rules and DOM rendering. A node added to the contract appears here without
// this file being touched, and a node that is not in the contract cannot appear here at all.
//
// StarterKit is still here for the things that are behaviour rather than schema: undo and redo,
// the drop cursor, the gap cursor, and the list backspace and delete handling. Everything it
// contributes to the schema is switched off, `trailingNode` included, because that one appends an
// empty paragraph to the end of a document and an editor that changes a file it was only asked to
// open is the one thing this app must never do.

import type { Extensions } from "@tiptap/core";
import { Proofing, SearchHighlight, createEngine } from "@margin/core/editor";
import { marks, nodes } from "../model/schema";
import "../store/useProofing";
import { BLOCK_EXTENSIONS } from "./blocks";
import { LinkPicker } from "./linkPicker";
import { imageView } from "./images";
import { createPaste, type PasteContext } from "./paste";
import { Shortcuts } from "./shortcuts";

/** Enter at the end of a finished task starts an unfinished one. Every other attribute carries. */
const RESET_ON_SPLIT: ReadonlySet<string> = new Set(["taskItem.checked"]);

export function createEditorExtensions(context: PasteContext): Extensions {
  return createEngine({
    nodes,
    marks,
    nodeViews: { image: ({ node }) => imageView(node) },
    resetOnSplit: RESET_ON_SPLIT,
    placeholder: (node) => {
      if (node.type.name === "heading") return `Heading ${node.attrs.level}`;
      if (node.type.name === "codeBlock") {
        return node.attrs.language === "mermaid" ? "graph TD" : "Start writing…";
      }
      if (node.type.name === "raw") return "";
      return "Start writing…";
    },
    extensions: [
      SearchHighlight,
      Proofing,
      createPaste(context),
      Shortcuts,
      LinkPicker.configure({ documentPath: context.documentPath }),
      ...BLOCK_EXTENSIONS,
    ],
  });
}
