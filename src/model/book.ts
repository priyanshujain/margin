import type { JSONContent } from "@tiptap/core";

export type TrimSize = "6x9" | "5.5x8.5" | "5x8" | "a5";
export type FigurePlacement = "inline" | "full-width" | "full-page" | "float-top";

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
}

export interface Chapter {
  id: string;
  title: string;
  content: JSONContent;
}

export interface Book {
  schema: "margin/1";
  id: string;
  metadata: BookMetadata;
  theme: string;
  settings: BookSettings;
  chapters: Chapter[];
}

function paragraph(text: string): JSONContent {
  return { type: "paragraph", content: [{ type: "text", text }] };
}

function emptyDoc(): JSONContent {
  return { type: "doc", content: [{ type: "paragraph" }] };
}

export function createChapter(title = "Untitled chapter"): Chapter {
  return { id: crypto.randomUUID(), title, content: emptyDoc() };
}

export function createBook(): Book {
  return {
    schema: "margin/1",
    id: crypto.randomUUID(),
    metadata: { title: "Untitled", subtitle: "", author: "", isbn: "", language: "en" },
    theme: "quiet-press",
    settings: { trim: "6x9", bleed: true },
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
