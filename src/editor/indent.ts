import { Extension } from "@tiptap/core";

export const ParagraphIndent = Extension.create({
  name: "paragraphIndent",

  addGlobalAttributes() {
    return [
      {
        types: ["paragraph"],
        attributes: {
          indent: {
            default: false,
            keepOnSplit: false,
            parseHTML: (el) => el.getAttribute("data-indent") === "true",
            renderHTML: (attrs) => (attrs.indent ? { "data-indent": "true" } : {}),
          },
        },
      },
    ];
  },

  addKeyboardShortcuts() {
    return {
      Tab: () => {
        if (this.editor.isActive("listItem")) return false;
        this.editor.commands.updateAttributes("paragraph", { indent: true });
        return true;
      },
      "Shift-Tab": () => {
        if (this.editor.isActive("listItem")) return false;
        this.editor.commands.updateAttributes("paragraph", { indent: false });
        return true;
      },
      Backspace: () => {
        const { $from, empty } = this.editor.state.selection;
        if (!empty || $from.parentOffset !== 0) return false;
        if ($from.parent.type.name !== "paragraph" || !$from.parent.attrs.indent) return false;
        return this.editor.commands.updateAttributes("paragraph", { indent: false });
      },
    };
  },
});
