import type { JSONContent } from "@tiptap/core";
import { type BookFonts, DEFAULT_FONTS } from "./fonts";

export type TrimSize = "6x9" | "5.5x8.5" | "5x8" | "a5";
export type FigurePlacement = "inline" | "full-width" | "full-page" | "float-top";

export const FIGURE_WIDTH: Record<FigurePlacement, number> = {
  inline: 62,
  "full-width": 100,
  "full-page": 100,
  "float-top": 46,
};

export const isResizablePlacement = (placement: FigurePlacement): boolean =>
  placement === "inline" || placement === "float-top";

export interface BookMetadata {
  title: string;
  subtitle: string;
  author: string;
  isbn: string;
  language: string;
}

export interface BookSettings {
  trim: TrimSize;
  bleed: boolean;
  fonts: BookFonts;
}

export type CoverKind = "default" | "image";

export interface Cover {
  kind: CoverKind;
  image: string;
  bg: string;
  ink: string;
}

export interface CoverPalette {
  id: string;
  label: string;
  bg: string;
  ink: string;
}

export const COVER_PALETTES: CoverPalette[] = [
  { id: "paper", label: "Paper", bg: "#f4efe3", ink: "#23201b" },
  { id: "ink", label: "Ink", bg: "#23201b", ink: "#f3ecdd" },
  { id: "forest", label: "Forest", bg: "#2c382f", ink: "#eae3d2" },
  { id: "oxblood", label: "Oxblood", bg: "#4a2327", ink: "#f0e4d8" },
  { id: "navy", label: "Navy", bg: "#222e3e", ink: "#e9e3d4" },
];

export const TRIM_DIMS: Record<TrimSize, { w: number; h: number }> = {
  "6x9": { w: 6, h: 9 },
  "5.5x8.5": { w: 5.5, h: 8.5 },
  "5x8": { w: 5, h: 8 },
  a5: { w: 148, h: 210 },
};

export function trimRatio(trim: TrimSize): number {
  const { w, h } = TRIM_DIMS[trim];
  return w / h;
}

export type ChapterKind = "front" | "body" | "part" | "back";

export interface Chapter {
  id: string;
  title: string;
  content: JSONContent;
  updatedAt: number;
  kind?: ChapterKind;
  noTitle?: boolean;
}

export interface PageType {
  id: string;
  label: string;
  group: "front" | "back";
}

export const PAGE_TYPES: PageType[] = [
  { id: "dedication", label: "Dedication", group: "front" },
  { id: "epigraph", label: "Epigraph", group: "front" },
  { id: "foreword", label: "Foreword", group: "front" },
  { id: "preface", label: "Preface", group: "front" },
  { id: "introduction", label: "Introduction", group: "front" },
  { id: "prologue", label: "Prologue", group: "front" },
  { id: "acknowledgments", label: "Acknowledgments", group: "front" },
  { id: "epilogue", label: "Epilogue", group: "back" },
  { id: "afterword", label: "Afterword", group: "back" },
  { id: "appendix", label: "Appendix", group: "back" },
  { id: "about-author", label: "About the Author", group: "back" },
  { id: "other", label: "New page", group: "front" },
];

export function chapterKind(chapter: Chapter): ChapterKind {
  return chapter.kind ?? "body";
}

export function bodyNumber(chapters: Chapter[], index: number): number | null {
  if (index < 0 || index >= chapters.length || chapterKind(chapters[index]) !== "body") return null;
  let n = 0;
  for (let i = 0; i <= index; i++) if (chapterKind(chapters[i]) === "body") n++;
  return n;
}

export function partNumber(chapters: Chapter[], index: number): number | null {
  if (index < 0 || index >= chapters.length || chapterKind(chapters[index]) !== "part") return null;
  let n = 0;
  for (let i = 0; i <= index; i++) if (chapterKind(chapters[i]) === "part") n++;
  return n;
}

const ROMAN: [number, string][] = [
  [1000, "M"], [900, "CM"], [500, "D"], [400, "CD"], [100, "C"], [90, "XC"],
  [50, "L"], [40, "XL"], [10, "X"], [9, "IX"], [5, "V"], [4, "IV"], [1, "I"],
];

export function partRoman(n: number): string {
  if (n < 1) return String(n);
  let num = n;
  let out = "";
  for (const [value, sym] of ROMAN) {
    while (num >= value) {
      out += sym;
      num -= value;
    }
  }
  return out;
}

export interface Book {
  schema: "margin/1";
  id: string;
  metadata: BookMetadata;
  theme: string;
  settings: BookSettings;
  cover: Cover;
  chapters: Chapter[];
}

export const SCHEMA_VERSION = 1;

export function schemaVersion(schema: unknown): number {
  if (typeof schema !== "string") return 0;
  const match = /^margin\/(\d+)$/.exec(schema);
  return match ? Number(match[1]) : 0;
}

function paragraph(text: string): JSONContent {
  return { type: "paragraph", content: [{ type: "text", text }] };
}

function emptyDoc(): JSONContent {
  return { type: "doc", content: [{ type: "paragraph" }] };
}

export function createChapter(title = "Untitled chapter"): Chapter {
  return { id: crypto.randomUUID(), title, content: emptyDoc(), updatedAt: Date.now() };
}

export function createPage(group: "front" | "back", title: string): Chapter {
  return { id: crypto.randomUUID(), title, content: emptyDoc(), kind: group, updatedAt: Date.now() };
}

export function createPart(title = ""): Chapter {
  return { id: crypto.randomUUID(), title, content: emptyDoc(), kind: "part", updatedAt: Date.now() };
}

export function cloneChapter(chapter: Chapter): Chapter {
  const title = chapter.noTitle || !chapter.title ? chapter.title : `${chapter.title} (copy)`;
  return { ...chapter, id: crypto.randomUUID(), title, content: structuredClone(chapter.content), updatedAt: Date.now() };
}

export function createCover(): Cover {
  return { kind: "default", image: "", bg: COVER_PALETTES[0].bg, ink: COVER_PALETTES[0].ink };
}

const DEFAULT_METADATA: BookMetadata = { title: "Untitled", subtitle: "", author: "", isbn: "", language: "en" };
const DEFAULT_SETTINGS: BookSettings = { trim: "6x9", bleed: true, fonts: DEFAULT_FONTS };

export function normalizeBook(book: Book): Book {
  const chapters = (Array.isArray(book.chapters) ? book.chapters : []).filter(Boolean);
  return {
    ...book,
    metadata: { ...DEFAULT_METADATA, ...book.metadata },
    settings: { ...DEFAULT_SETTINGS, ...book.settings, fonts: { ...DEFAULT_FONTS, ...book.settings?.fonts } },
    cover: book.cover ? { ...createCover(), ...book.cover } : createCover(),
    chapters: (chapters.length ? chapters : [createChapter()]).map((c) =>
      c.updatedAt ? c : { ...c, updatedAt: Date.now() },
    ),
  };
}

export function createBook(): Book {
  return {
    schema: "margin/1",
    id: crypto.randomUUID(),
    metadata: { ...DEFAULT_METADATA },
    theme: "quiet-press",
    settings: { ...DEFAULT_SETTINGS },
    cover: createCover(),
    chapters: [createChapter("Chapter One")],
  };
}

export function starterBook(): Book {
  const book = createBook();
  book.metadata.title = "The Lighthouse";
  book.chapters = [
    {
      id: crypto.randomUUID(),
      title: "The Lighthouse",
      updatedAt: Date.now(),
      content: {
        type: "doc",
        content: [
          paragraph(
            "The lamp had not been lit in thirty years, and yet the islanders still set their clocks by a light that no longer came. Mara climbed the iron stair each evening out of a habit older than memory, and each evening she found the same dark glass waiting, patient as the sea."
          ),
          paragraph(
            "Her grandfather had kept the flame. Her mother had let it die. Between those two facts lay the whole of the family's quarrel with the water, and Mara had inherited both the quarrel and the key that opened the lantern room."
          ),
          { type: "horizontalRule" },
          paragraph(
            "Morning came the color of pewter. The tide had left its usual accounting along the shingle, and Mara walked the length of it without hurry, naming each thing as her grandfather had named them to her."
          ),
        ],
      },
    },
    createChapter("Salt and Iron"),
    createChapter("What the Tide Left"),
  ];
  return book;
}
