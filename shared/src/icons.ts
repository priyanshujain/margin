// The title bar glyphs, drawn on a 24 unit grid for a 1.6 stroke.
//
// Here because the two apps kept drifting. Each had its own idea of what a search or a moon looked
// like, they were adjusted independently, and the result was two products from the same hand that
// did not look related. A path is a design decision, not a detail, and the fix for two copies of a
// decision is one copy.
//
// Paths and not an icon dependency: a set is six hundred kilobytes for the handful of shapes a
// title bar needs, and every one of these is a few dozen bytes.
//
// Each app renders these through its own `Icon` component. The two components are identical today
// and are deliberately not shared: one is React, which would make this package depend on React for
// twenty four lines, and a component is where an app is entitled to differ.

/** A pane and its divider. Also the preview dock in Margin, mirrored. */
export const SIDEBAR = "M3 4.5h18v15H3zM9 4.5v15";

/** Margin's preview dock: the same pane with the divider on the other side. */
export const DOCK = "M3 4.5h18v15H3zM14 4.5v15";

export const SEARCH = "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4";

/** A capital A with a tick beside it: the letter a checker is looking at, checked. */
export const SPELLING = "M4 17l4-10 4 10M5.4 13.4h5.2M15 17l2.5 2.5L22 14";

/** Three lines of a paragraph and a squiggle under the last, which is the mark grammar leaves. */
export const GRAMMAR = "M4 7h16M4 12h12M4 17h7M13.5 18.5c1-1.2 2-1.2 3 0s2 1.2 3 0";

/**
 * A capital A beside a lowercase a, which is what a font panel has been called since there were
 * font panels.
 *
 * Two letterforms and not one on a rule: a letter over a full width line is the underline button in
 * every editor anybody has used. The other constraint is SPELLING above, which is also built on a
 * capital A; what separates them is the shape to its right, a round bowl here and an angular tick
 * there, and that difference survives 16px in a way a crossbar's height would not.
 */
export const FONT =
  "M2.5 18L6.5 6l4 12M4.3 14.2h4.4M17 11.3a3.2 3.2 0 1 0 0 6.4a3.2 3.2 0 0 0 0-6.4M20.2 11.3v6.7";

/** The page's two edges with a double headed arrow between them. */
export const WIDTH = "M3 5v14M21 5v14M7 12h10M7 12l3-3M7 12l3 3M17 12l-3-3M17 12l-3 3";

export const EXPORT = "M5 13v6h14v-6M12 16V3M8 7l4-4 4 4";

export const MOON = "M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z";

/**
 * The sun, which is the one glyph here that is not a single path: the disc has to be a circle so
 * that it stays round at every size, and the rays have to be a path so that they keep their caps.
 * Rendered as two children rather than one `d`.
 */
export const SUN_RAYS =
  "M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M19.1 4.9l-1.4 1.4M6.3 17.7l-1.4 1.4";
export const SUN_DISC = { cx: 12, cy: 12, r: 4 } as const;

export const MORE = "M5 12h.01M12 12h.01M19 12h.01";

export const CLOSE = "M6 6l12 12M18 6L6 18";

export const CHECK = "M20 6L9 17l-5-5";
