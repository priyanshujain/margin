import { createOverlays } from "@margin/core/hooks";

/**
 * Which overlay is on screen. Nothing here is resident: an overlay is summoned by a key or a
 * hover and dismissed with Escape. Only one is open at a time, so the keymap's context stack has
 * exactly one frame to shadow the grid's bindings with.
 *
 * `trail` is where you came from. Settings opens Accounts, and without it that panel is a dead end
 * whose only way out is closing everything and starting again.
 */
export type Overlay =
  | "palette"
  | "search"
  | "mini-month"
  | "editor"
  | "shortcuts"
  /** The phone's overflow sheet. A desktop reaches all of this from the header row or a key. */
  | "menu";

export const useOverlays = createOverlays<Overlay>();
