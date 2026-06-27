import { Extension } from "@tiptap/core";

export type Alignment = "left" | "center" | "right";

const ALIGNED_TYPES = ["paragraph", "heading"];

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    textAlign: {
      setTextAlign: (align: Alignment) => ReturnType;
    };
  }
}

export const TextAlign = Extension.create({
  name: "textAlign",

  addGlobalAttributes() {
    return [
      {
        types: ALIGNED_TYPES,
        attributes: {
          align: {
            default: null,
            keepOnSplit: true,
            parseHTML: (el) => {
              const value = el.style.textAlign || el.getAttribute("data-align");
              return value === "center" || value === "right" ? value : null;
            },
            renderHTML: (attrs) =>
              attrs.align ? { "data-align": attrs.align, style: `text-align: ${attrs.align}` } : {},
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      setTextAlign:
        (align) =>
        ({ tr, dispatch }) => {
          const value = align === "left" ? null : align;
          const { from, to } = tr.selection;
          let changed = false;
          tr.doc.nodesBetween(from, to, (node, pos) => {
            if (ALIGNED_TYPES.includes(node.type.name) && node.attrs.align !== value) {
              if (dispatch) tr.setNodeAttribute(pos, "align", value);
              changed = true;
            }
          });
          return changed;
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      "Mod-Shift-l": () => this.editor.commands.setTextAlign("left"),
      "Mod-Shift-e": () => this.editor.commands.setTextAlign("center"),
      "Mod-Shift-r": () => this.editor.commands.setTextAlign("right"),
    };
  },
});
