import { Node, mergeAttributes } from "@tiptap/core";
import { ReactNodeViewRenderer } from "@tiptap/react";
import { isResizablePlacement } from "../model/book";
import { FigureView } from "./FigureView";

export const Figure = Node.create({
  name: "figure",
  group: "block",
  atom: true,
  draggable: true,
  selectable: true,

  addAttributes() {
    return {
      src: { default: "" },
      alt: { default: "" },
      caption: { default: "" },
      placement: { default: "full-width" },
      width: { default: null },
    };
  },

  parseHTML() {
    return [{ tag: "figure[data-figure]" }];
  },

  renderHTML({ HTMLAttributes }) {
    const { src, alt, caption, placement, width } = HTMLAttributes;
    const style = width != null && isResizablePlacement(placement) ? `width:${width}%` : null;
    return [
      "figure",
      mergeAttributes({ "data-figure": "", "data-placement": placement, style }),
      ["img", { src, alt }],
      ["figcaption", {}, caption ?? ""],
    ];
  },

  addNodeView() {
    return ReactNodeViewRenderer(FigureView);
  },
});
