export type CoreContext = "global" | "view" | "editor" | "overlay";

export type KeyContext = CoreContext | (string & {});

export type CommandId = string;

interface BindingBase {
  keys: readonly string[];
  context: KeyContext;
  group: string;
  allowInInput?: boolean;
}

export interface CommandBinding extends BindingBase {
  command: CommandId;
  label: string;
  palette?: boolean;
}

export interface NoteBinding extends BindingBase {
  command: null;
  label: string;
}

export type Binding = CommandBinding | NoteBinding;

export const APP_GROUP = "App";

export const COMMAND_PALETTE = "command-palette";
export const SHORTCUTS = "shortcuts";
export const SETTINGS = "settings";
export const QUICK_OPEN = "quick-open";
export const TOGGLE_THEME = "toggle-theme";

export interface SuiteOptions {
  quickOpen?: boolean;
}

export function suiteBindings({ quickOpen = false }: SuiteOptions = {}): Binding[] {
  return [
    {
      keys: ["cmd+k", "cmd+shift+p"],
      command: COMMAND_PALETTE,
      label: "Command palette",
      context: "global",
      group: APP_GROUP,
      allowInInput: true,
    },
    ...(quickOpen
      ? [
          {
            keys: ["cmd+p"],
            command: QUICK_OPEN,
            label: "Quick open",
            context: "view",
            group: APP_GROUP,
            allowInInput: true,
            palette: true,
          } satisfies CommandBinding,
        ]
      : []),
    {
      keys: ["?", "shift+?", "cmd+/"],
      command: SHORTCUTS,
      label: "Keyboard shortcuts",
      context: "view",
      group: APP_GROUP,
      palette: true,
    },
    {
      keys: ["Escape"],
      command: null,
      label: "Dismiss whatever is open",
      context: "global",
      group: APP_GROUP,
    },
    {
      keys: ["cmd+,"],
      command: SETTINGS,
      label: "Settings",
      context: "view",
      group: APP_GROUP,
      allowInInput: true,
      palette: true,
    },
    {
      keys: [],
      command: TOGGLE_THEME,
      label: "Toggle dark mode",
      context: "view",
      group: APP_GROUP,
      palette: true,
    },
  ];
}

const isMac =
  typeof navigator !== "undefined" && /mac|iphone|ipad/i.test(navigator.userAgent ?? "");

export const PRIMARY_LABEL = isMac ? "⌘" : "Ctrl+";

export const primaryHeld = (e: { metaKey: boolean; ctrlKey: boolean }): boolean =>
  isMac ? e.metaKey : e.ctrlKey;

export const secondaryHeld = (e: { metaKey: boolean; ctrlKey: boolean }): boolean =>
  isMac ? e.ctrlKey : e.metaKey;

export function normalizeCombo(combo: string): string {
  const parts = combo.split("+");
  let key = parts.pop() ?? "";
  if (key === "" && parts.length > 0) key = "+";
  const mods = new Set(parts.filter((p) => p !== "").map((p) => p.toLowerCase()));

  if (key.length === 1 && /[a-z]/i.test(key)) {
    if (mods.size === 0 && key !== key.toLowerCase()) mods.add("shift");
    key = key.toLowerCase();
  } else if (key.length === 1) {
    key = key.toLowerCase();
  }

  const prefix = ["cmd", "ctrl", "alt", "shift"].filter((m) => mods.has(m)).join("+");
  return prefix ? `${prefix}+${key}` : key;
}

const NAMED: Record<string, string> = {
  Enter: "↩",
  Escape: "⎋",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Tab: "⇥",
  " ": "Space",
};

export function keyLabel(combo: string): string {
  const parts = normalizeCombo(combo).split("+");
  let key = parts.pop() ?? "";
  if (key === "" && parts.length > 0) key = "+";
  const printed = parts
    .map((m) => (m === "cmd" ? PRIMARY_LABEL : m === "ctrl" ? "⌃" : m === "shift" ? "⇧" : "⌥"))
    .join("");
  const named = NAMED[key];
  if (named) return `${printed}${named}`;
  return `${printed}${key.length === 1 ? key.toUpperCase() : key}`;
}

const tables: (readonly Binding[])[] = [];
let version = 0;
let flat: readonly Binding[] = [];

export function registerBindings(table: readonly Binding[]): () => void {
  tables.push(table);
  version += 1;
  flat = tables.flat();
  return () => {
    const at = tables.lastIndexOf(table);
    if (at === -1) return;
    tables.splice(at, 1);
    version += 1;
    flat = tables.flat();
  };
}

export function bindingsVersion(): number {
  return version;
}

export function allBindings(): readonly Binding[] {
  return flat;
}

export function groupsOf(bindings: readonly Binding[] = flat): string[] {
  return [...new Set(bindings.map((b) => b.group))];
}

export function keysFor(id: CommandId): readonly string[] {
  return flat.find((b) => b.command === id)?.keys ?? [];
}

export function keyFor(id: CommandId): string | null {
  const keys = keysFor(id);
  return keys.length > 0 ? keys[0] : null;
}

export function labelFor(id: CommandId): string {
  return flat.find((b) => b.command === id)?.label ?? id;
}

export function paletteCommands(): CommandBinding[] {
  return flat.filter((b): b is CommandBinding => b.command !== null && b.palette === true);
}
