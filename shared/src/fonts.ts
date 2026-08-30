// The faces both apps offer, and the two slots they set them into.
//
// This lives in one place because the two apps have to agree about it. A face named here is a
// `@font-face` in css/fonts.css, a file in fonts/, and a family a Typst preamble names on the way
// to a PDF, and those four lists going out of step with each other is a document that renders in
// one app and falls back to Georgia in the other. There is no way to keep four lists in two repos
// honest by hand, so there is one list.
//
// Two slots and not one. Body and heading are the only typographic decision worth a control:
// a document that lets its author pick a face per paragraph is a word processor, and neither of
// these is one. The scale, the leading and the measure belong to each app's own stylesheet, which
// decided them once for every document.
//
// A `FontRef` is stored, not a family name. "Literata" as a string cannot say whether it means the
// file in fonts/ or a copy the user installed themselves, and those are two different faces the
// moment one of them is updated.

export type FontCategory = "serif" | "sans" | "display";

/** One face that ships in fonts/, in the variable file both editors render from. */
export interface BundledFont {
  id: string;
  label: string;
  /** The CSS family name, which is also what a Typst preamble names it by. */
  family: string;
  category: FontCategory;
  regular: string;
  italic: string;
  /** The weight range the variable file covers, for the `@font-face` in css/fonts.css. */
  weight: string;
}

/**
 * The six, and the order a picker lists them in: the serifs a page of prose is set in, the one
 * display face, then the sans both apps' own chrome already uses.
 *
 * Literata and Hanken Grotesk are first-class here and also special: they are the two that have
 * static instances cut for PDF export, so they are the only pair whose bold really is bold on the
 * page. The other four export as their variable file at one weight, which each app's pdf.rs
 * explains.
 */
export const BUNDLED_FONTS: readonly BundledFont[] = [
  {
    id: "literata",
    label: "Literata",
    family: "Literata",
    category: "serif",
    regular: "Literata-VF.ttf",
    italic: "Literata-Italic-VF.ttf",
    weight: "200 900",
  },
  {
    id: "eb-garamond",
    label: "EB Garamond",
    family: "EB Garamond",
    category: "serif",
    regular: "EBGaramond-VF.ttf",
    italic: "EBGaramond-Italic-VF.ttf",
    weight: "400 800",
  },
  {
    id: "lora",
    label: "Lora",
    family: "Lora",
    category: "serif",
    regular: "Lora-VF.ttf",
    italic: "Lora-Italic-VF.ttf",
    weight: "400 700",
  },
  {
    id: "source-serif",
    label: "Source Serif 4",
    family: "Source Serif 4",
    category: "serif",
    regular: "SourceSerif4-VF.ttf",
    italic: "SourceSerif4-Italic-VF.ttf",
    weight: "200 900",
  },
  {
    id: "fraunces",
    label: "Fraunces",
    family: "Fraunces",
    category: "display",
    regular: "Fraunces-VF.ttf",
    italic: "Fraunces-Italic-VF.ttf",
    weight: "100 900",
  },
  {
    id: "hanken",
    label: "Hanken Grotesk",
    family: "Hanken Grotesk",
    category: "sans",
    regular: "HankenGrotesk-VF.ttf",
    italic: "HankenGrotesk-Italic-VF.ttf",
    weight: "100 900",
  },
];

/** One of the six, or a family off the machine. */
export type FontRef = { kind: "bundled"; id: string } | { kind: "system"; family: string };

/**
 * What one book or one document is set in.
 *
 * Named for the pair rather than for either app's noun, because the two call the thing it belongs
 * to different names. Each app aliases it: `BookFonts` in Margin, `DocumentFonts` in Margin Docs.
 */
export interface FontPair {
  body: FontRef;
  heading: FontRef;
}

/**
 * A named pair, which is what a picker offers first.
 *
 * Pairing two faces is the part of this that takes an eye, and a list of twelve families with no
 * opinion attached is how a document ends up in Fraunces body text. The presets are the answer to
 * "make this look like something"; the two selects underneath are for somebody who already knows.
 */
export interface FontPairing {
  id: string;
  label: string;
  body: FontRef;
  heading: FontRef;
}

const bundled = (id: string): FontRef => ({ kind: "bundled", id });

export const FONT_PAIRINGS: readonly FontPairing[] = [
  { id: "quiet-press", label: "Quiet Press", body: bundled("literata"), heading: bundled("literata") },
  { id: "classic", label: "Classic", body: bundled("eb-garamond"), heading: bundled("eb-garamond") },
  { id: "editorial", label: "Editorial", body: bundled("source-serif"), heading: bundled("fraunces") },
  { id: "modern", label: "Modern", body: bundled("lora"), heading: bundled("hanken") },
  { id: "contrast", label: "Contrast", body: bundled("literata"), heading: bundled("hanken") },
  { id: "plain", label: "Plain", body: bundled("hanken"), heading: bundled("hanken") },
];

/** What a page is set in until somebody says otherwise: the css/tokens.css pair, in FontRef form. */
export const DEFAULT_FONTS: FontPair = { body: bundled("literata"), heading: bundled("literata") };

// The two tails from css/tokens.css, so a face that fails to load falls back to what the app would
// have used anyway rather than to the webview's default.
const SERIF_FALLBACK = `Georgia, "Times New Roman", serif`;
const SANS_FALLBACK = `ui-sans-serif, system-ui, -apple-system, sans-serif`;

export function bundledFont(id: string): BundledFont | undefined {
  return BUNDLED_FONTS.find((f) => f.id === id);
}

/**
 * A `FontRef` as one string, which is what a `<select>` value and a stored preference both need.
 *
 * The two-character tag is the whole point: a system family can be called "Literata" and must not
 * come back as the bundled one.
 */
export function encodeRef(ref: FontRef): string {
  return ref.kind === "bundled" ? `b:${ref.id}` : `s:${ref.family}`;
}

export function decodeRef(value: string): FontRef {
  return value.startsWith("s:")
    ? { kind: "system", family: value.slice(2) }
    : { kind: "bundled", id: value.slice(2) };
}

/** The bare family name, which is what a Typst preamble names a face by. */
export function fontFamilyName(ref: FontRef): string {
  if (ref.kind === "system") return ref.family;
  return bundledFont(ref.id)?.family ?? "Literata";
}

/** What a picker calls it. Identical to the family for a system face, which has no other name. */
export function fontLabel(ref: FontRef): string {
  if (ref.kind === "system") return ref.family;
  return bundledFont(ref.id)?.label ?? ref.id;
}

/** A CSS font stack, which is what goes into `--font-book` and `--font-heading`. */
export function fontStack(ref: FontRef): string {
  if (ref.kind === "bundled") {
    const font = bundledFont(ref.id);
    if (!font) return `"Literata", ${SERIF_FALLBACK}`;
    return `"${font.family}", ${font.category === "sans" ? SANS_FALLBACK : SERIF_FALLBACK}`;
  }
  return `"${ref.family}", ${SERIF_FALLBACK}`;
}

export function refsEqual(a: FontRef, b: FontRef): boolean {
  if (a.kind === "bundled" && b.kind === "bundled") return a.id === b.id;
  if (a.kind === "system" && b.kind === "system") return a.family === b.family;
  return false;
}

export function fontsEqual(a: FontPair, b: FontPair): boolean {
  return refsEqual(a.body, b.body) && refsEqual(a.heading, b.heading);
}

/** Which preset this pair is, or null for a combination somebody built themselves. */
export function pairingFor(fonts: FontPair): string | null {
  const match = FONT_PAIRINGS.find(
    (p) => refsEqual(p.body, fonts.body) && refsEqual(p.heading, fonts.heading),
  );
  return match?.id ?? null;
}

/**
 * The faces an export has to be handed, split by where their bytes come from.
 *
 * Bundled faces come back as ids rather than as records, because the id is what crosses the IPC
 * boundary: the backend already has the bytes compiled in and looks them up by id. Deduplicated,
 * because the common case is one family in both slots and asking the backend to load a two megabyte
 * variable file twice is two megabytes of IPC for nothing.
 */
export function fontsUsed(fonts: FontPair): { bundled: string[]; system: string[] } {
  const usedBundled = new Set<string>();
  const usedSystem = new Set<string>();
  for (const ref of [fonts.body, fonts.heading]) {
    if (ref.kind === "bundled") {
      if (bundledFont(ref.id)) usedBundled.add(ref.id);
    } else {
      usedSystem.add(ref.family);
    }
  }
  return { bundled: [...usedBundled], system: [...usedSystem] };
}

/** Anything that is not a `FontRef`, from a stored preference written by another version. */
export function isFontRef(value: unknown): value is FontRef {
  if (typeof value !== "object" || value === null) return false;
  const ref = value as { kind?: unknown; id?: unknown; family?: unknown };
  if (ref.kind === "bundled") return typeof ref.id === "string" && bundledFont(ref.id) !== undefined;
  return ref.kind === "system" && typeof ref.family === "string" && ref.family !== "";
}
