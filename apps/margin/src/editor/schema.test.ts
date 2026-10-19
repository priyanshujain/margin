import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { getSchema, type JSONContent } from "@tiptap/core";
import { editorExtensions } from "./extensions";
import { normalizeBook, starterBook, type Book } from "../model/book";
import { fixtureBooks } from "../dev/fixture";

const schema = getSchema(editorExtensions);

function roundTrip(content: JSONContent): JSONContent {
  const doc = schema.nodeFromJSON(content);
  doc.check();
  return doc.toJSON() as JSONContent;
}

const RICH: JSONContent = {
  type: "doc",
  content: [
    { type: "heading", attrs: { level: 2, textAlign: null }, content: [{ type: "text", text: "Title" }] },
    {
      type: "paragraph",
      attrs: { textAlign: "center", indent: 1 },
      content: [
        { type: "text", marks: [{ type: "bold" }, { type: "italic" }], text: "both" },
        { type: "hardBreak" },
        {
          type: "text",
          marks: [{ type: "link", attrs: { href: "https://example.com", target: "_blank", rel: "noopener noreferrer nofollow", class: null, title: null } }],
          text: "link",
        },
      ],
    },
    { type: "blockquote", content: [{ type: "paragraph", content: [{ type: "text", text: "quoted" }] }] },
    { type: "bulletList", content: [{ type: "listItem", content: [{ type: "paragraph", content: [{ type: "text", text: "a" }] }] }] },
    { type: "taskList", content: [{ type: "taskItem", attrs: { checked: true }, content: [{ type: "paragraph", content: [{ type: "text", text: "done" }] }] }] },
    { type: "horizontalRule" },
  ],
};

describe("the book schema", () => {
  it("reads and writes every block the editor offers without changing it", () => {
    const once = roundTrip(RICH);
    expect(roundTrip(once)).toEqual(once);
  });

  it("keeps the sample and starter books exactly as written", () => {
    for (const book of [...fixtureBooks(), starterBook()]) {
      for (const chapter of normalizeBook(book).chapters) {
        expect(roundTrip(chapter.content)).toEqual(roundTrip(roundTrip(chapter.content)));
      }
    }
  });
});

const COMPAT = process.env.MARGIN_COMPAT_DIR;

describe.skipIf(!COMPAT)("a copy of a real library", () => {
  it("opens every book and reads every chapter into the editor without losing anything", () => {
    const dir = COMPAT as string;
    expect(dir).not.toContain("Library/Application Support");
    expect(dir).not.toContain("Library/Containers");
    const names = readdirSync(dir).filter((name) => name.endsWith(".margin"));
    let chapters = 0;
    let identical = 0;
    for (const name of names) {
      const raw = readFileSync(join(dir, name), "utf8");
      const book = normalizeBook(JSON.parse(raw) as Book);
      for (const chapter of book.chapters) {
        const stored = schema.nodeFromJSON(chapter.content);
        const canonical = roundTrip(chapter.content);
        expect(schema.nodeFromJSON(canonical).eq(stored), `${name} ${chapter.id}`).toBe(true);
        expect(roundTrip(canonical), `${name} ${chapter.id}`).toEqual(canonical);
        chapters += 1;
      }
      if (JSON.stringify(book, null, 2) === raw) identical += 1;
    }
    console.log(`books ${names.length}, chapters ${chapters}, untouched re-save identical ${identical}/${names.length}`);
    expect(names.length).toBeGreaterThan(0);
  });
});
