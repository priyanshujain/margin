import { createMenuHandler } from "@margin/core/keys";
import type { CommandId } from "./commands";

/** Exactly the ids `src-tauri/src/lib.rs` emits. */
export const MENU_IDS: readonly CommandId[] = [
  "open-folder",
  "new-doc",
  "new-folder",
  "save",
  "export-pdf",
  "close-folder",
  "settings",
  "find",
  "find-in-files",
  "quick-open",
  "command-palette",
  "toggle-sidebar",
  "toggle-outline",
  "check-updates",
  "report-issue",
  "writing-proofread",
  "writing-rewrite",
];

export const handleMenuAction = createMenuHandler(MENU_IDS);
