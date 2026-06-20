import type { JSONContent } from "@tiptap/core";
import type { Book, TrimSize } from "../model/book";
import type { ImageInput } from "../ipc";

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

function figure(node: JSONContent, paths: Map<string, string>): string {
  const path = paths.get(node.attrs?.src);
  if (!path) return "";
  const placement = node.attrs?.placement ?? "inline";
  if (placement === "full-page") {
    return `#page(margin: 0pt, numbering: none)[#image(${str(path)}, width: 100%, height: 100%, fit: "cover")]`;
  }
  const caption = node.attrs?.caption ? `, caption: [${esc(node.attrs.caption)}]` : "";
  const float = placement === "float-top" ? ", placement: top" : "";
  return `#figure(image(${str(path)}, width: 100%)${caption}${float})`;
}

function block(node: JSONContent, paths: Map<string, string>): string {
  switch (node.type) {
    case "paragraph":
      return guardLineStart(inlines(node.content));
    case "heading":
      return `#heading(level: ${node.attrs?.level ?? 2})[${inlines(node.content)}]`;
    case "blockquote":
      return `#blockquote[${(node.content ?? []).map((n) => block(n, paths)).join("\n\n")}]`;
    case "bulletList":
      return (node.content ?? []).map((li) => `- ${listItem(li)}`).join("\n");
    case "orderedList":
      return (node.content ?? []).map((li) => `+ ${listItem(li)}`).join("\n");
    case "horizontalRule":
      return "#scenebreak";
    case "figure":
      return figure(node, paths);
    default:
      return "";
  }
}

function chapterBody(content: JSONContent, paths: Map<string, string>): string {
  return (content.content ?? [])
    .map((n) => block(n, paths))
    .filter((s) => s.length > 0)
    .join("\n\n");
}

function preamble(book: Book): string {
  const trim = TRIM[book.settings.trim];
  const meta = book.metadata;
  const bleed = book.settings.bleed ? "0.125in" : "0in";
  return `#set document(title: ${str(meta.title || "Untitled")}, author: ${str(meta.author)})
#set page(
  width: ${trim.w} + ${bleed},
  height: ${trim.h} + ${bleed} * 2,
  margin: (inside: 0.875in, outside: 0.625in + ${bleed}, top: 0.8in + ${bleed}, bottom: 0.8in + ${bleed}),
  binding: left,
)
#set text(font: "Literata", size: 11pt, lang: ${str(meta.language || "en")}, hyphenate: true)
#set par(justify: true, leading: 0.72em, spacing: 0.72em, first-line-indent: (amount: 1.3em, all: false))
#show heading: set text(font: "Literata", weight: "medium")
#show heading.where(level: 1): set text(size: 22pt)
#show heading.where(level: 2): set block(above: 1.4em, below: 0.6em)
#show heading.where(level: 3): set block(above: 1.2em, below: 0.5em)

#let scenebreak = align(center)[#v(0.5em) #line(length: 13%, stroke: 0.5pt + rgb("#d6cfbd")) #v(0.5em)]

#let blockquote(body) = pad(left: 1.2em)[#set text(style: "italic", fill: rgb("#6b6458")); #body]

#let titlepage(title, subtitle, author) = {
  v(2.4in)
  align(center)[
    #text(font: "Literata", size: 30pt, weight: "medium")[#title]
    #if subtitle != "" {
      v(0.6em)
      text(font: "Literata", size: 15pt, style: "italic", fill: rgb("#6b6458"))[#subtitle]
    }
    #if author != "" {
      v(1.7em)
      text(font: "Hanken Grotesk", size: 10.5pt, weight: "medium", tracking: 0.22em)[#upper(author)]
    }
  ]
}

#let chapteropener(num, title) = {
  pagebreak(weak: true)
  v(2.1in)
  align(center)[
    #text(font: "Hanken Grotesk", size: 8.5pt, weight: "semibold", tracking: 0.28em)[#upper("Chapter " + num)]
    #v(0.7em)
    #heading(level: 1, numbering: none, outlined: true)[#title]
  ]
  v(1.5em)
}`;
}

function imageExtension(dataUrl: string): string {
  const match = /^data:image\/([a-z0-9.+-]+)/i.exec(dataUrl);
  const kind = (match?.[1] ?? "png").toLowerCase();
  return kind === "jpeg" ? "jpg" : kind;
}

export function extractImages(book: Book): { images: ImageInput[]; paths: Map<string, string> } {
  const paths = new Map<string, string>();
  const images: ImageInput[] = [];

  const visit = (node: JSONContent) => {
    if (node.type === "figure") {
      const src: string | undefined = node.attrs?.src;
      if (src && src.startsWith("data:") && !paths.has(src)) {
        const path = `assets/figure-${images.length + 1}.${imageExtension(src)}`;
        paths.set(src, path);
        images.push({ path, data: src.slice(src.indexOf(",") + 1) });
      }
    }
    (node.content ?? []).forEach(visit);
  };

  book.chapters.forEach((chapter) => visit(chapter.content));
  return { images, paths };
}

export function bookToTypst(book: Book, paths: Map<string, string> = new Map()): string {
  const meta = book.metadata;
  const front = `#set page(numbering: none)
#titlepage(${str(meta.title || "Untitled")}, ${str(meta.subtitle)}, ${str(meta.author)})
#pagebreak(weak: true)
#outline(title: [Contents], depth: 1)
#set page(numbering: "1")
#counter(page).update(1)`;

  const body = book.chapters
    .map((chapter, i) => {
      const opener = `#chapteropener(${str(String(i + 1))}, [${esc(chapter.title || "Untitled")}])`;
      return `${opener}\n\n${chapterBody(chapter.content, paths)}`;
    })
    .join("\n\n");

  return `${preamble(book)}\n\n${front}\n\n${body}\n`;
}

export function bookToPdfInputs(book: Book): { source: string; images: ImageInput[] } {
  const { images, paths } = extractImages(book);
  return { source: bookToTypst(book, paths), images };
}
