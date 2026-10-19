import {
  APP_GROUP,
  COMMAND_PALETTE,
  SETTINGS,
  SHORTCUTS,
  TOGGLE_THEME,
  registerBindings,
  suiteBindings,
  type Binding,
  type CommandBinding,
  type NoteBinding,
} from "@margin/core/keys";

export { COMMAND_PALETTE, SETTINGS, SHORTCUTS, TOGGLE_THEME };

export const GROUPS = ["Projects", "Navigation", "Editing", "Formatting", APP_GROUP] as const;

type Group = (typeof GROUPS)[number];

function command(
  id: string,
  label: string,
  group: Group,
  keys: string[] = [],
  palette = true,
): CommandBinding {
  return { command: id, label, group, keys, context: "view", allowInInput: true, palette };
}

function note(label: string, keys: string[], group: Group = "Formatting"): NoteBinding {
  return { command: null, label, keys, group, context: "editor" };
}

export const APP_BINDINGS: readonly Binding[] = [
  { command: SHORTCUTS, label: "Keyboard shortcuts", keys: ["cmd+/", "cmd+shift+/", "cmd+shift+?"], group: "unlisted", context: "view", allowInInput: true, palette: false },
  ...suiteBindings(),
  command("new-book", "New project", "Projects", ["cmd+n"]),
  command("import-epub", "Import EPUB…", "Projects"),
  command("check-updates", "Check for updates", APP_GROUP),
  command("report-issue", "Report an issue", APP_GROUP),
];

export const BOOK_BINDINGS: readonly Binding[] = [
  command("save", "Save", "Projects", ["cmd+s"]),
  command("export-pdf", "Export PDF…", "Projects", ["cmd+alt+p", "cmd+alt+π"]),
  command("export-epub", "Export EPUB…", "Projects", ["cmd+shift+e"]),
  command("project-setup", "Project setup", "Projects"),
  command("all-projects", "All projects", "Projects"),
  command("find", "Find", "Navigation", ["cmd+f"]),
  command("find-replace", "Find and replace", "Navigation", ["cmd+alt+f", "cmd+alt+ƒ"]),
  command("toggle-chapters", "Toggle chapters", "Navigation", ["cmd+\\"]),
  command("toggle-preview", "Toggle preview", "Navigation"),
  command("next-chapter", "Next chapter", "Navigation", ["cmd+alt+ArrowDown", "cmd+alt+ArrowRight", "ctrl+Tab"]),
  command("prev-chapter", "Previous chapter", "Navigation", ["cmd+alt+ArrowUp", "cmd+alt+ArrowLeft", "ctrl+shift+Tab"]),
  command("toggle-spelling", "Check spelling", "Editing"),
  command("toggle-grammar", "Check grammar", "Editing"),
  command("writing-proofread", "Proofread with Writing Tools", "Editing", ["alt+shift+f", "alt+shift+Ï"]),
  command("writing-rewrite", "Rewrite with Writing Tools", "Editing", ["alt+shift+r", "alt+shift+‰"]),
  note("Paste without formatting", ["cmd+shift+v"], "Editing"),
  note("Bold", ["cmd+b"]),
  note("Italic", ["cmd+i"]),
  note("Underline", ["cmd+u"]),
  note("Strikethrough", ["cmd+shift+x"]),
  note("Add or edit link", ["cmd+shift+k"]),
  note("Heading", ["cmd+alt+2", "cmd+alt+3"]),
  note("Paragraph", ["cmd+alt+0"]),
  note("Bullet list", ["cmd+shift+8"]),
  note("Numbered list", ["cmd+shift+7"]),
  note("Task list", ["cmd+shift+9"]),
  note("Quote", ["cmd+shift+b"]),
  note("Align left", ["cmd+shift+l"]),
  note("Align right", ["cmd+shift+r"]),
];

registerBindings(APP_BINDINGS);

export function registerBookBindings(): () => void {
  return registerBindings(BOOK_BINDINGS);
}

export const MENU_COMMANDS: Readonly<Record<string, string>> = {
  "new-book": "new-book",
  save: "save",
  "export-pdf": "export-pdf",
  "export-epub": "export-epub",
  "command-palette": COMMAND_PALETTE,
  "check-updates": "check-updates",
  settings: SETTINGS,
  find: "find",
  "toggle-chapters": "toggle-chapters",
  "next-chapter": "next-chapter",
  "prev-chapter": "prev-chapter",
  "report-issue": "report-issue",
  "keyboard-shortcuts": SHORTCUTS,
  "writing-proofread": "writing-proofread",
  "writing-rewrite": "writing-rewrite",
};
