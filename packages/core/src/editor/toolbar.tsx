// Toolbar items as data. Each app lists the items it shows and lays them out its own way (Docs' pill
// at the foot of the page, Margin's floating bar); what an item is called, what it draws and how its
// button behaves is decided here once.

import type { ReactElement, ReactNode } from "react";

export const TOOL_ICONS = {
  bold: "M7 5v14M7 5h5.5a3.5 3.5 0 0 1 0 7H7M7 12h6a3.5 3.5 0 0 1 0 7H7",
  italic: "M10 5h6M6 19h6M13 5l-4 14",
  strikethrough:
    "M5 12h14M8 7.5c0-1.5 1.6-2.5 4-2.5s4 1 4 2.5M8 16.5c0 1.5 1.6 2.5 4 2.5s4-1 4-2.5",
  code: "M9 6l-5 6 5 6M15 6l5 6-5 6",
  heading: "M7 5v14M7 12h10M17 5v14",
  bulletList: "M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01",
  orderedList:
    "M10 6h11M10 12h11M10 18h11M4 4v4M3 4h2M4 10.5h1.5a1 1 0 1 1 0 2H4h1.5a1 1 0 1 1 0 2H4M4 20.5l1.4-1.7a1 1 0 1 0-1.4-1.6",
  taskList: "M4 5h4v4H4zM5.5 7l1 1 2-2M4 15h4v4H4zM5 17l1 1 2-2M11 7h9M11 17h9M11 12h9",
  blockquote: "M7 8h4v4a4 4 0 0 1-4 4M14 8h4v4a4 4 0 0 1-4 4",
  toggle: "M5 7l4 5-4 5M12 9h7M12 15h7",
  codeBlock: "M4 6h16v12H4zM7 10l3 2-3 2",
  link: "M10 13a5 5 0 0 0 7 0l2-2a5 5 0 0 0-7-7l-1 1M14 11a5 5 0 0 0-7 0l-2 2a5 5 0 0 0 7 7l1-1",
  remove: "M18 6L6 18M6 6l12 12",
  check: "M5 12.5l4.5 4.5L19 7",
  rule: "M5 12h5M14 12h5",
  image: "M4 5h16v14H4zM4 16l4.5-4.5 3 3L16 10l4 4",
  alert: "M12 3l10 18H2zM12 9v5M12 17h.01",
  callout: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18M12 11v5M12 8h.01",
  table: "M4 5h16v14H4zM4 10h16M10 10v9M15 10v9",
  insert: "M12 5v14M5 12h14",
  color: "M6 15l6-11 6 11M8.2 11h7.6M5 20h14",
} as const;

export type ToolIcon = keyof typeof TOOL_ICONS;

export interface ToolbarItem {
  id: string;
  label: string;
  icon: ToolIcon;
}

export const TOOLBAR_ITEMS = {
  bold: { id: "bold", label: "Bold", icon: "bold" },
  italic: { id: "italic", label: "Italic", icon: "italic" },
  strikethrough: { id: "strikethrough", label: "Strikethrough", icon: "strikethrough" },
  code: { id: "code", label: "Inline code", icon: "code" },
  heading: { id: "heading", label: "Heading", icon: "heading" },
  bulletList: { id: "bulletList", label: "Bulleted list", icon: "bulletList" },
  orderedList: { id: "orderedList", label: "Numbered list", icon: "orderedList" },
  taskList: { id: "taskList", label: "Task list", icon: "taskList" },
  blockquote: { id: "blockquote", label: "Quote", icon: "blockquote" },
  callout: { id: "callout", label: "Callout", icon: "callout" },
  toggle: { id: "toggle", label: "Toggle", icon: "toggle" },
  codeBlock: { id: "codeBlock", label: "Code block", icon: "codeBlock" },
  link: { id: "link", label: "Link", icon: "link" },
  rule: { id: "rule", label: "Horizontal rule", icon: "rule" },
  image: { id: "image", label: "Insert image", icon: "image" },
  table: { id: "table", label: "Table", icon: "table" },
  color: { id: "color", label: "Text colour", icon: "color" },
} as const satisfies Record<string, ToolbarItem>;

/**
 * One toolbar button. It never takes focus from the document (the mousedown is cancelled), and a
 * glyph-only button carries its name in aria-label while the tip shows the chord for the eye.
 */
export function tool(
  active: boolean,
  onClick: () => void,
  label: string,
  content: ReactNode,
  disabled = false,
  chord?: string,
): ReactElement {
  return (
    <button
      className="tool"
      data-on={active}
      data-tip={chord ? `${label} (${chord})` : label}
      aria-label={label}
      disabled={disabled}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
    >
      {content}
    </button>
  );
}
