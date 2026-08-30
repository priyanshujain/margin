// The faces a book can be set in.
//
// The catalogue itself is margin-shared's, because Margin and Margin Docs have to offer the same
// six families under the same ids: a book and a document set in "EB Garamond" must mean the same
// file, and the two apps drifting apart on which faces exist is how one of them ends up exporting a
// PDF in a fallback. What is left here is the one thing that is this app's own, which is the noun.

export type {
  BundledFont,
  FontCategory,
  FontPairing,
  FontRef,
} from "margin-shared/fonts";

export {
  BUNDLED_FONTS,
  DEFAULT_FONTS,
  FONT_PAIRINGS,
  bundledFont,
  decodeRef,
  encodeRef,
  fontFamilyName,
  fontLabel,
  fontStack,
  fontsEqual,
  fontsUsed,
  isFontRef,
  pairingFor,
  refsEqual,
} from "margin-shared/fonts";

import type { FontPair } from "margin-shared/fonts";

/**
 * What one book is set in.
 *
 * An alias rather than a second interface, so every call site in this app keeps reading in the
 * app's own noun while the shape stays the shared one. Margin Docs calls the same pair
 * `DocumentFonts`.
 */
export type BookFonts = FontPair;
