import type { JSONContent } from "@tiptap/core";
import type { Book, TrimSize } from "../model/book";

const TRIM: Record<TrimSize, { w: string; h: string }> = {
  "6x9": { w: "6in", h: "9in" },
  "5.5x8.5": { w: "5.5in", h: "8.5in" },
  "5x8": { w: "5in", h: "8in" },
  a5: { w: "148mm", h: "210mm" },
};

const INLINE_SPECIAL = /[\\#$*_`<>@~[\]]/g;

function esc(text: string): string {
  return text.replace(INLINE_SPECIAL, (m) => "\\" + m);
}

function guardLineStart(line: string): string {
  if (/^[=\-+/]/.test(line)) return "\\" + line;
  if (/^\d+\./.test(line)) return line.replace(".", "\\.");
  return line;
}

function str(value: string): string {
  return JSON.stringify(value);
}

function inline(node: JSONContent): string {
  if (node.type === "text") {
    let text = esc(node.text ?? "");
    const marks = (node.marks ?? []).map((m) => m.type);
    if (marks.includes("italic")) text = `#emph[${text}]`;
    if (marks.includes("bold")) text = `#strong[${text}]`;
    return text;
  }
  if (node.type === "hardBreak") return "#linebreak()";
  return "";
}

function inlines(content: JSONContent[] = []): string {
  return content.map(inline).join("");
}

function listItem(item: JSONContent): string {
  return (item.content ?? [])
    .filter((c) => c.type === "paragraph")
    .map((p) => inlines(p.content))
    .join(" ");
}

function block(node: JSONContent): string {
  switch (node.type) {
    case "paragraph":
      return guardLineStart(inlines(node.content));
    case "heading":
      return `#heading(level: ${node.attrs?.level ?? 2})[${inlines(node.content)}]`;
    case "blockquote":
      return `#blockquote[${(node.content ?? []).map(block).join("\n\n")}]`;
    case "bulletList":
      return (node.content ?? []).map((li) => `- ${listItem(li)}`).join("\n");
    case "orderedList":
      return (node.content ?? []).map((li) => `+ ${listItem(li)}`).join("\n");
    case "horizontalRule":
      return "#scenebreak";
    default:
      return "";
  }
}

function chapterBody(content: JSONContent): string {
  return (content.content ?? [])
    .map(block)
    .filter((s) => s.length > 0)
    .join("\n\n");
}

function preamble(book: Book): string {
  const trim = TRIM[book.settings.trim];
  const meta = book.metadata;
  return `#set document(title: ${str(meta.title || "Untitled")}, author: ${str(meta.author)})
#set page(
  width: ${trim.w},
  height: ${trim.h},
  margin: (inside: 0.875in, outside: 0.625in, top: 0.8in, bottom: 0.8in),
  binding: left,
  numbering: "1",
)
#set text(font: "Literata", size: 11pt, lang: ${str(meta.language || "en")}, hyphenate: true)
#set par(justify: true, leading: 0.72em, spacing: 0.72em, first-line-indent: (amount: 1.3em, all: false))
#show heading: set text(font: "Literata", weight: "medium")
#show heading: set block(above: 1.4em, below: 0.8em)

#let scenebreak = align(center)[#v(0.5em) #line(length: 13%, stroke: 0.5pt + rgb("#d6cfbd")) #v(0.5em)]

#let blockquote(body) = pad(left: 1.2em, rest: 0pt)[#set text(style: "italic", fill: rgb("#6b6458")); #body]

#let chapteropener(num, title) = {
  pagebreak(weak: true, to: "odd")
  v(2.1in)
  align(center)[
    #text(font: "Hanken Grotesk", size: 8.5pt, weight: "semibold", tracking: 0.28em)[#upper("Chapter " + num)]
    #v(0.7em)
    #text(font: "Literata", size: 22pt, weight: "medium")[#title]
  ]
  v(1.5em)
}`;
}

export function bookToTypst(book: Book): string {
  const body = book.chapters
    .map((chapter, i) => {
      const opener = `#chapteropener(${str(String(i + 1))}, [${esc(chapter.title || "Untitled")}])`;
      return `${opener}\n\n${chapterBody(chapter.content)}`;
    })
    .join("\n\n");
  return `${preamble(book)}\n\n${body}\n`;
}
