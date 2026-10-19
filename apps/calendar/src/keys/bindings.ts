// The keymap, declared once, and the command ids it dispatches.
//
// It is registered with margin-core's keymap, which dispatches from it, renders the `?` sheet from
// it and lists it in the palette, so a binding that exists but is undocumented is not something
// you can write. The suite's own keys (the palette on Cmd+K and Cmd+Shift+P, `?`, Cmd+, and the
// theme toggle) come from core's `suiteBindings` and are registered after this table.
//
// `cmd` means the platform's primary modifier, Command on macOS and Control everywhere else. A
// capital letter is the shifted one, so `H` reads as Shift+H. Nothing is chorded and nothing is
// modal.

import {
  registerBindings,
  suiteBindings,
  type Binding,
  type CommandId,
} from "@margin/core/keys/bindings";

export type { CommandId };

export type BindingGroup = "Navigation" | "Views" | "Events" | "App";

export const GROUPS: readonly BindingGroup[] = ["Navigation", "Views", "Events", "App"];

export const BINDINGS: readonly Binding[] = [
  { keys: ["h"], command: "day-prev", label: "Previous day", context: "view", group: "Navigation" },
  { keys: ["l"], command: "day-next", label: "Next day", context: "view", group: "Navigation" },
  { keys: ["H"], command: "week-prev", label: "Previous week", context: "view", group: "Navigation" },
  { keys: ["L"], command: "week-next", label: "Next week", context: "view", group: "Navigation" },
  {
    keys: ["t", "cmd+t"],
    command: "today",
    label: "Go to today",
    context: "view",
    group: "Navigation",
    palette: true,
  },
  { keys: ["j"], command: "select-next", label: "Select the next event", context: "view", group: "Navigation" },
  { keys: ["k"], command: "select-prev", label: "Select the previous event", context: "view", group: "Navigation" },
  {
    keys: ["m"],
    command: "mini-month",
    label: "Jump to a date",
    context: "view",
    group: "Navigation",
    palette: true,
  },

  { keys: ["d", "cmd+1"], command: "view-day", label: "Day view", context: "view", group: "Views", palette: true },
  { keys: ["w", "cmd+2"], command: "view-week", label: "Week view", context: "view", group: "Views", palette: true },
  {
    keys: ["a", "cmd+3"],
    command: "view-agenda",
    label: "Agenda view",
    context: "view",
    group: "Views",
    palette: true,
  },
  { keys: ["z"], command: "fold", label: "Fold or unfold the band under the cursor", context: "view", group: "Views" },
  {
    keys: ["Z"],
    command: "unfold-all",
    label: "Unfold every band",
    context: "view",
    group: "Views",
    palette: true,
  },

  { keys: ["c", "cmd+n"], command: "new-event", label: "New event", context: "view", group: "Events", palette: true },
  { keys: ["Enter"], command: "open-selection", label: "Open the selected event", context: "view", group: "Events" },
  { keys: ["e"], command: "edit-selection", label: "Edit the selected event", context: "view", group: "Events" },
  { keys: ["x"], command: "delete-selection", label: "Delete the selected event", context: "view", group: "Events" },
  { keys: ["/", "cmd+f"], command: "search", label: "Search events", context: "view", group: "Events", palette: true },

  { keys: ["cmd+r"], command: "sync-now", label: "Sync now", context: "view", group: "App", palette: true },
  { keys: [], command: "calendars", label: "Calendars", context: "view", group: "App", palette: true },
  { keys: [], command: "accounts", label: "Accounts", context: "view", group: "App", palette: true },
  { keys: [], command: "check-updates", label: "Check for updates", context: "view", group: "App", palette: true },
  { keys: [], command: "report-issue", label: "Report an issue", context: "view", group: "App", palette: true },
];

/** Exactly the ids `src-tauri/src/lib.rs` emits as `menu-action`. */
export const MENU_COMMAND_IDS: readonly CommandId[] = [
  "new-event",
  "command-palette",
  "sync-now",
  "accounts",
  "check-updates",
  "settings",
  "search",
  "today",
  "shortcuts",
  "view-day",
  "view-week",
  "view-agenda",
  "report-issue",
];

registerBindings(BINDINGS);
registerBindings(suiteBindings());
