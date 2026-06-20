import { Node, mergeAttributes } from "@tiptap/core";
import { ReactNodeViewRenderer } from "@tiptap/react";
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
    };
  },

  parseHTML() {
    return [{ tag: "figure[data-figure]" }];
  },

  renderHTML({ HTMLAttributes }) {
    const { src, alt, caption, placement } = HTMLAttributes;
    return [
      "figure",
      mergeAttributes({ "data-figure": "", "data-placement": placement }),
      ["img", { src, alt }],
      ["figcaption", {}, caption ?? ""],
    ];
  },

  addNodeView() {
    return ReactNodeViewRenderer(FigureView);
  },
});
