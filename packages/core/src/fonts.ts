export type FontCategory = "serif" | "sans" | "display";

export interface BundledFont {
  id: string;
  label: string;
  family: string;
  category: FontCategory;
  regular: string;
  italic: string;
  weight: string;
}

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

export type FontRef = { kind: "bundled"; id: string } | { kind: "system"; family: string };

export interface FontPair {
  body: FontRef;
  heading: FontRef;
}

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

export const DEFAULT_FONTS: FontPair = { body: bundled("literata"), heading: bundled("literata") };

const SERIF_FALLBACK = `Georgia, "Times New Roman", serif`;
const SANS_FALLBACK = `ui-sans-serif, system-ui, -apple-system, sans-serif`;

export function bundledFont(id: string): BundledFont | undefined {
  return BUNDLED_FONTS.find((f) => f.id === id);
}

export function encodeRef(ref: FontRef): string {
  return ref.kind === "bundled" ? `b:${ref.id}` : `s:${ref.family}`;
}

export function decodeRef(value: string): FontRef {
  return value.startsWith("s:")
    ? { kind: "system", family: value.slice(2) }
    : { kind: "bundled", id: value.slice(2) };
}

export function fontFamilyName(ref: FontRef): string {
  if (ref.kind === "system") return ref.family;
  return bundledFont(ref.id)?.family ?? "Literata";
}

export function fontLabel(ref: FontRef): string {
  if (ref.kind === "system") return ref.family;
  return bundledFont(ref.id)?.label ?? ref.id;
}

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

export function pairingFor(fonts: FontPair): string | null {
  const match = FONT_PAIRINGS.find(
    (p) => refsEqual(p.body, fonts.body) && refsEqual(p.heading, fonts.heading),
  );
  return match?.id ?? null;
}

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

export function isFontRef(value: unknown): value is FontRef {
  if (typeof value !== "object" || value === null) return false;
  const ref = value as { kind?: unknown; id?: unknown; family?: unknown };
  if (ref.kind === "bundled") return typeof ref.id === "string" && bundledFont(ref.id) !== undefined;
  return ref.kind === "system" && typeof ref.family === "string" && ref.family !== "";
}
