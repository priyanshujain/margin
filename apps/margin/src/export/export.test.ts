import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { Window } from "happy-dom";
import { normalizeBook, type Book } from "../model/book";
import { fixtureBooks } from "../dev/fixture";
import { bookToTypst, unsupportedScripts } from "./typst";
import { bookToEpub } from "./epub";
import { filesToBook } from "../import/epub";

const NO_FONTS = { files: [], faces: "", manifest: [] };

function book(): Book {
  const base = normalizeBook(fixtureBooks()[0]);
  return { ...base, metadata: { ...base.metadata, title: 'Rivers "and" #tides $5', author: "A \\ B" } };
}

describe("typst export", () => {
  it("writes metadata as escaped string literals and bold as strong", () => {
    const source = bookToTypst(book());
    expect(source).toContain('#titlepage("Rivers \\"and\\" #tides $5", "", "A \\\\ B")');
    expect(source).toContain("#strong[bold]");
    expect(source).toContain("#emph[italic]");
  });

  it("escapes markup characters in prose", () => {
    const b = book();
    b.chapters[0].content = {
      type: "doc",
      content: [{ type: "paragraph", content: [{ type: "text", text: "Costs $5 #1 @me *not bold*" }] }],
    };
    const source = bookToTypst(b);
    expect(source).toContain("Costs \\$5 \\#1 \\@me \\*not bold\\*");
  });

  it("flags scripts the bundled faces cannot set", () => {
    const b = book();
    b.chapters[0].content = { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text: "日本語" }] }] };
    expect(unsupportedScripts(b).length).toBeGreaterThan(0);
    expect(unsupportedScripts(book())).toEqual([]);
  });
});

describe("epub export and import", () => {
  const window = new Window();
  beforeAll(() => {
    globalThis.DOMParser = window.DOMParser as unknown as typeof DOMParser;
  });
  afterAll(async () => {
    await window.happyDOM.close();
  });

  it("packages a valid container and reads its own output back", () => {
    const original = book();
    const files = bookToEpub(original, NO_FONTS);
    const paths = files.map((f) => f.path);
    expect(paths).toContain("META-INF/container.xml");
    expect(paths.some((p) => p.endsWith(".opf"))).toBe(true);
    const xhtml = files.filter((f) => f.path.endsWith(".xhtml")).map((f) => f.data).join("\n");
    expect(xhtml).toContain("<strong>bold</strong>");
    expect(xhtml).toContain("Rivers &quot;and&quot; #tides $5");

    const imported = filesToBook(files.map((f) => ({ path: f.path, data: f.data, encoding: f.encoding })) as never);
    expect(imported.metadata.title).toBe('Rivers "and" #tides $5');
    const titles = imported.chapters.map((c) => c.title);
    expect(titles).toEqual(expect.arrayContaining(["Morning", "Evening"]));
  });
});
