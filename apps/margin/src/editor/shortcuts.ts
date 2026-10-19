import { Extension } from "@tiptap/core";

export const Shortcuts = Extension.create({
  name: "shortcuts",

  addKeyboardShortcuts() {
    return {
      "Mod-Shift-x": () => this.editor.commands.toggleStrike(),
    };
  },
});
