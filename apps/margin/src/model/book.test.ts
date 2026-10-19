import { describe, expect, it } from "vitest";
import {
  bodyNumber,
  chapterKind,
  createBook,
  createChapter,
  createPage,
  createPart,
  insertByKind,
  normalizeBook,
  partNumber,
  partRoman,
  schemaVersion,
  starterBook,
  SCHEMA_VERSION,
} from "./book";

describe("book model", () => {
  it("normalising twice changes nothing", () => {
    const once = normalizeBook(starterBook());
    expect(normalizeBook(once)).toEqual(once);
    expect(JSON.stringify(normalizeBook(once), null, 2)).toBe(JSON.stringify(once, null, 2));
  });

  it("fills what an old file leaves out without touching what it has", () => {
    const sparse = { schema: "margin/1", id: "x", metadata: { title: "Old" }, chapters: [] } as never;
    const book = normalizeBook(sparse);
    expect(book.metadata.title).toBe("Old");
    expect(book.metadata.language).toBe("en");
    expect(book.chapters).toHaveLength(1);
    expect(book.settings.fonts.body).toBeDefined();
    expect(book.cover).toBeDefined();
  });

  it("reads the schema version and refuses nothing it knows", () => {
    expect(schemaVersion(createBook().schema)).toBe(SCHEMA_VERSION);
    expect(schemaVersion("margin/2")).toBe(2);
  });

  it("keeps front matter, body and back matter in their order", () => {
    const front = createPage("front", "Dedication");
    const back = createPage("back", "About the author");
    const one = createChapter("One");
    let chapters = insertByKind([one], back);
    chapters = insertByKind(chapters, front);
    chapters = insertByKind(chapters, createChapter("Two"));
    expect(chapters.map(chapterKind)).toEqual(["front", "body", "body", "back"]);
  });

  it("numbers chapters and parts the way the sidebar and the PDF do", () => {
    const chapters = [createPage("front", "Preface"), createPart("Beginnings"), createChapter("A"), createChapter("B")];
    expect(bodyNumber(chapters, 2)).toBe(1);
    expect(bodyNumber(chapters, 3)).toBe(2);
    expect(partNumber(chapters, 1)).toBe(1);
    expect(partRoman(4)).toBe("IV");
    expect(partRoman(14)).toBe("XIV");
  });
});
