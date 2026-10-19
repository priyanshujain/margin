// Docs' keymap, declared once on margin-core's engine. The core keymap dispatches from this table
// and the shortcuts sheet and command palette render from it.
//
// Combos are core's form: `cmd` is the platform's primary modifier and Shift is a modifier like any
// other, so Cmd+Shift+F is `cmd+shift+f`. The suite set (palette, quick open, shortcuts, settings,
// theme) comes from core's `suiteBindings`; everything else is Docs' own.

import {
  APP_GROUP,
  COMMAND_PALETTE,
  QUICK_OPEN,
  SETTINGS,
  SHORTCUTS,
  TOGGLE_THEME,
  registerBindings,
  registerCommands,
  suiteBindings,
  type Binding,
  type CommandBinding,
} from "@margin/core/keys";
import { COMMANDS, commandLabel, runCommand, type CommandId } from "./commands";

export { PRIMARY_LABEL, keyLabel, keysFor, normalizeCombo } from "@margin/core/keys";

export type BindingGroup = "File" | "Navigation" | "Search" | "Editing" | "View" | typeof APP_GROUP;

export const GROUPS: readonly BindingGroup[] = ["File", "Navigation", "Search", "Editing", "View", APP_GROUP];

const SUITE: ReadonlySet<CommandId> = new Set<CommandId>([
  COMMAND_PALETTE,
  QUICK_OPEN,
  SHORTCUTS,
  SETTINGS,
  TOGGLE_THEME,
] as CommandId[]);

type Keyed = { keys: readonly string[]; group: BindingGroup; inInput?: boolean };

const KEYS: Partial<Record<CommandId, Keyed>> = {
  "open-folder": { keys: ["cmd+o"], group: "File", inInput: true },
  "new-doc": { keys: ["cmd+n"], group: "File", inInput: true },
  save: { keys: ["cmd+s"], group: "File", inInput: true },
  "export-pdf": { keys: ["cmd+shift+e"], group: "File", inInput: true },

  "previous-document": { keys: ["cmd+["], group: "Navigation", inInput: true },
  "next-document": { keys: ["cmd+]"], group: "Navigation", inInput: true },

  find: { keys: ["cmd+f"], group: "Search", inInput: true },
  "find-in-files": { keys: ["cmd+shift+f"], group: "Search", inInput: true },

  // Cmd+K is the suite's palette, so the link tool takes the shifted chord.
  "insert-link": { keys: ["cmd+shift+k"], group: "Editing", inInput: true },
  "underline-unsupported": { keys: ["cmd+u"], group: "Editing", inInput: true },
  "correct-spelling": { keys: ["cmd+;"], group: "Editing", inInput: true },
  "writing-proofread": { keys: ["alt+shift+f"], group: "Editing", inInput: true },
  "writing-rewrite": { keys: ["alt+shift+r"], group: "Editing", inInput: true },

  "toggle-sidebar": { keys: ["cmd+\\"], group: "View", inInput: true },
  // A keyboard reports the bar with Shift held; a synthetic event may not set the flag.
  "toggle-outline": { keys: ["cmd+shift+|", "cmd+|"], group: "View", inInput: true },
};

const GROUP_OF: Partial<Record<CommandId, BindingGroup>> = {
  "new-folder": "File",
  "close-folder": "File",
  "rename-file": "File",
  "duplicate-file": "File",
  "delete-file": "File",
  "reveal-in-finder": "File",
  "editor-width-tight": "View",
  "editor-width-narrow": "View",
  "editor-width-normal": "View",
  "editor-width-wide": "View",
  "editor-width-full": "View",
  "toggle-spelling": "Editing",
  "toggle-grammar": "Editing",
};

function docsBinding(id: CommandId, palette: boolean): CommandBinding {
  const keyed = KEYS[id];
  return {
    keys: keyed?.keys ?? [],
    command: id,
    label: commandLabel(id),
    context: "view",
    group: keyed?.group ?? GROUP_OF[id] ?? APP_GROUP,
    allowInInput: keyed?.inInput,
    palette,
  };
}

export const BINDINGS: readonly Binding[] = [
  ...suiteBindings({ quickOpen: true }),
  ...COMMANDS.filter((c) => !SUITE.has(c.id)).map((c) => docsBinding(c.id, c.palette)),
];

registerBindings(BINDINGS);
registerCommands(Object.fromEntries(COMMANDS.map((c) => [c.id, () => runCommand(c.id)])));
