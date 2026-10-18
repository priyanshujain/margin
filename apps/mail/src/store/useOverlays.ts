import { createOverlays } from "@margin/core/hooks";

/**
 * Which overlay is on screen. Ported from the calendar's store of the same name, trail included.
 *
 * Nothing here is resident: an overlay is summoned by a key and dismissed with Escape, and only one
 * is open at a time, so the keymap's context stack has exactly one frame to shadow the view's
 * bindings with.
 *
 * `trail` is where you came from. The palette opens Settings, and without it that panel is a dead
 * end whose only way out is closing everything and starting again.
 *
 * The list names every panel the app will have, not only the ones that exist today: `compose`,
 * `settings` and `accounts` belong to later packages and nothing opens them yet.
 */
export type Overlay =
  | "palette"
  | "shortcuts"
  /** The slideshow a newly added account ends on, and the first row of the help menu after that. */
  | "tour"
  /** The guide, which takes the window rather than the stage: it is read over the app, not instead
      of it, and closing it puts back exactly what was underneath. */
  | "guide"
  | "search"
  | "compose"
  | "settings"
  | "accounts"
  | "snooze"
  /** The phone's overflow sheet. A desktop reaches all of this from the header or a key. */
  | "menu";

export const useOverlays = createOverlays<Overlay>();
