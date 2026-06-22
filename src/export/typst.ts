import type { JSONContent } from "@tiptap/core";
import { type Book, type FigurePlacement, type TrimSize, FIGURE_WIDTH, bodyNumber, chapterKind, isResizablePlacement } from "../model/book";
import type { ImageInput } from "../ipc";

const TRIM: Record<TrimSize, { w: string; h: string }> = {
  "6x9": { w: "6in", h: "9in" },
  "5.5x8.5": { w: "5.5in", h: "8.5in" },
  "5x8": { w: "5in", h: "8in" },
  a5: { w: "148mm", h: "210mm" },
};

const INLINE_SPECIAL = /[\\#$*_`<>@~[\]]/g;
const LINE_SEPARATORS = /[\n\r\v\f\u2028\u2029]+/g;

function esc(text: string): string {
  return text.replace(LINE_SEPARATORS, " ").replace(INLINE_SPECIAL, (m) => "\\" + m);
}

function guardLineStart(text: string): string {
  if (/^[=\-+/]/.test(text)) return "\\" + text;
  if (/^\d+\./.test(text)) return text.replace(".", "\\.");
  return text;
}

function str(value: string): string {
  return JSON.stringify(value);
}

function inline(node: JSONContent, atLineStart: boolean): string {
  if (node.type === "text") {
    const marks = node.marks ?? [];
    if (marks.some((m) => m.type === "code")) return `#raw(${str(node.text ?? "")})`;
    let text = esc(node.text ?? "");
    if (atLineStart) text = guardLineStart(text);
    if (marks.some((m) => m.type === "italic")) text = `#emph[${text}]`;
    if (marks.some((m) => m.type === "bold")) text = `#strong[${text}]`;
    if (marks.some((m) => m.type === "strike")) text = `#strike[${text}]`;
    if (marks.some((m) => m.type === "underline")) text = `#underline[${text}]`;
    const href = marks.find((m) => m.type === "link")?.attrs?.href;
    if (href) text = `#link(${str(href)})[${text}]`;
    return text;
  }
  if (node.type === "hardBreak") return "#linebreak()";
  return "";
}

function inlines(content: JSONContent[] = [], atLineStart = false): string {
  return content.map((node, i) => inline(node, atLineStart && i === 0)).join("");
}

function listItem(item: JSONContent, paths: Map<string, string>): string {
  const parts = (item.content ?? [])
    .map((child) => block(child, paths))
    .filter((s) => s.length > 0);
  return `[${parts.join("\n\n")}]`;
}

function figure(node: JSONContent, paths: Map<string, string>): string {
  const path = paths.get(node.attrs?.src);
  if (!path) return "";
  const placement = node.attrs?.placement ?? "inline";
  if (placement === "full-page") {
    return `#page(margin: 0pt, numbering: none)[#image(${str(path)}, width: 100%, height: 100%, fit: "cover")]`;
  }
  const placed = placement as FigurePlacement;
  const width = (isResizablePlacement(placed) ? node.attrs?.width : null) ?? FIGURE_WIDTH[placed] ?? 100;
  const caption = node.attrs?.caption ? `, caption: [${guardLineStart(esc(node.attrs.caption))}]` : "";
  const float = placement === "float-top" ? ", placement: top" : "";
  return `#figure(image(${str(path)}, width: ${width}%)${caption}${float})`;
}

function block(node: JSONContent, paths: Map<string, string>): string {
  switch (node.type) {
    case "paragraph": {
      const body = inlines(node.content, true);
      if (!body.trim()) return "~";
      return node.attrs?.indent ? `#h(1.3em)${body}` : body;
    }
    case "heading":
      return `#heading(level: ${node.attrs?.level ?? 2})[${inlines(node.content, true)}]`;
    case "blockquote":
      return `#blockquote[${(node.content ?? []).map((n) => block(n, paths)).join("\n\n")}]`;
    case "bulletList":
      return `#list(${(node.content ?? []).map((li) => listItem(li, paths)).join(", ")})`;
    case "orderedList":
      return `#enum(${(node.content ?? []).map((li) => listItem(li, paths)).join(", ")})`;
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
#set par(justify: true, leading: 0.72em, spacing: 0.72em)
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

#let openchapter(info) = {
  pagebreak(weak: true)
  [#metadata(info) <chap>]
  v(2.1in)
  align(center)[
    #if info.kind == "body" {
      text(font: "Hanken Grotesk", size: 8.5pt, weight: "semibold", tracking: 0.28em)[#upper("Chapter " + info.num)]
      if not info.notitle { v(0.7em) }
    }
    #if not info.notitle {
      heading(level: 1, numbering: none, outlined: info.toc)[#info.title]
    }
  ]
  v(1.5em)
}

#let pageof(loc) = {
  let pat = loc.page-numbering()
  if pat == none { pat = "1" }
  numbering(pat, ..counter(page).at(loc))
}

#let contents() = {
  pagebreak(weak: true)
  set par(justify: false, first-line-indent: 0pt)
  block(below: 2em)[
    #show heading: set block(above: 0pt, below: 0pt)
    #heading(level: 1, numbering: none, outlined: false, bookmarked: true)[#text(font: "Literata", size: 24pt, weight: "medium")[Contents]]
  ]
  context {
    let items = query(<chap>).filter(it => it.value.toc != false)
    let prev = none
    for it in items {
      let info = it.value
      let pg = pageof(it.location())
      let body = info.kind == "body"
      let gap = if prev != none and prev != info.kind { 1.7em } else { 0.95em }
      prev = info.kind
      block(width: 100%, above: gap, below: 0.95em, {
        set text(size: 11.5pt)
        link(it.location())[#grid(
          columns: (1.9em, 1fr, auto),
          column-gutter: (0.65em, 1em),
          align: (right + top, left + top, right + top),
          if body { info.num } else { [] },
          if body { info.title } else { emph(info.title) },
          pg,
        )]
      })
    }
  }
}`;
}

function imageExtension(dataUrl: string): string {
  const match = /^data:image\/([a-z0-9.+-]+)/i.exec(dataUrl);
  const kind = (match?.[1] ?? "png").toLowerCase().split("+")[0];
  return kind === "jpeg" ? "jpg" : kind;
}

function coverBlock(book: Book, coverPath?: string): string {
  const c = book.cover;
  const meta = book.metadata;

  if (c.kind === "image" && coverPath) {
    return `#page(margin: 0pt, numbering: none)[#image(${str(coverPath)}, width: 100%, height: 100%, fit: "cover")]`;
  }

  const subtitle = meta.subtitle
    ? `#v(0.55em)\n  #text(font: "Literata", size: 14pt, style: "italic")[${esc(meta.subtitle)}]`
    : "";
  const author = meta.author
    ? `#v(1fr)\n  #text(font: "Hanken Grotesk", size: 11pt, weight: "semibold", tracking: 0.24em)[#upper[${esc(meta.author)}]]\n  #v(0.55in)`
    : "#v(1fr)";

  return `#page(margin: 0pt, numbering: none, fill: rgb(${str(c.bg)}))[
  #set align(center)
  #set text(fill: rgb(${str(c.ink)}))
  #v(1fr)
  #text(font: "Literata", size: 30pt, weight: "medium")[${esc(meta.title || "Untitled")}]
  #v(0.5em)
  #line(length: 16%, stroke: 0.6pt + rgb(${str(c.ink)}))
  ${subtitle}
  ${author}
]`;
}

function pushCoverImage(book: Book, images: ImageInput[]): string | undefined {
  const c = book.cover;
  if (c.kind !== "image" || !c.image.startsWith("data:")) return undefined;
  const path = `assets/cover.${imageExtension(c.image)}`;
  images.push({ path, data: c.image.slice(c.image.indexOf(",") + 1) });
  return path;
}

function collectImages(contents: JSONContent[]): { images: ImageInput[]; paths: Map<string, string> } {
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

  contents.forEach(visit);
  return { images, paths };
}

export function extractImages(book: Book): { images: ImageInput[]; paths: Map<string, string> } {
  return collectImages(book.chapters.map((chapter) => chapter.content));
}

function cleanTitle(title: string): string {
  return (title || "")
    .replace(/[\u0000-\u001f\u007f-\u009f\u2028\u2029]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function normTitle(title: string): string {
  return cleanTitle(title).toLowerCase();
}

function inToc(book: Book, index: number): boolean {
  const chapter = book.chapters[index];
  if (chapter.noTitle && chapterKind(chapter) !== "body") return false;
  const title = cleanTitle(chapter.title) || "Untitled";
  return !(chapterKind(chapter) === "front" && normTitle(title) === normTitle(book.metadata.title));
}

function openerCall(book: Book, index: number): string {
  const chapter = book.chapters[index];
  const kind = chapterKind(chapter);
  const num = kind === "body" ? String(bodyNumber(book.chapters, index)) : "";
  const notitle = !!chapter.noTitle;
  const title = notitle ? "" : cleanTitle(chapter.title) || "Untitled";
  return `#openchapter((kind: ${str(kind)}, num: ${str(num)}, title: ${str(title)}, notitle: ${notitle}, toc: ${inToc(book, index)}))`;
}

export function bookToTypst(book: Book, paths: Map<string, string> = new Map(), coverPath?: string): string {
  const meta = book.metadata;
  const cover = `${coverBlock(book, coverPath)}\n#pagebreak(weak: true)`;
  const front = `#set page(numbering: none)
#titlepage(${str(meta.title || "Untitled")}, ${str(meta.subtitle)}, ${str(meta.author)})
#pagebreak(weak: true)
#set page(numbering: "i")
#counter(page).update(1)`;

  const firstBody = book.chapters.findIndex((c) => chapterKind(c) === "body");
  const firstToc = book.chapters.findIndex((_, i) => inToc(book, i));

  const body = book.chapters
    .map((chapter, i) => {
      const toc = i === firstToc ? `#contents()\n\n` : "";
      const reset = i === firstBody ? `#set page(numbering: "1")\n#counter(page).update(1)\n` : "";
      return `${toc}${reset}${openerCall(book, i)}\n\n${chapterBody(chapter.content, paths)}`;
    })
    .join("\n\n");

  return `${preamble(book)}\n\n${cover}\n\n${front}\n\n${body}\n`;
}

export function bookToPdfInputs(book: Book): { source: string; images: ImageInput[] } {
  const { images, paths } = extractImages(book);
  const coverPath = pushCoverImage(book, images);
  return { source: bookToTypst(book, paths, coverPath), images };
}

export function coverToPdfInputs(book: Book): { source: string; images: ImageInput[] } {
  const images: ImageInput[] = [];
  const coverPath = pushCoverImage(book, images);
  return { source: `${preamble(book)}\n\n${coverBlock(book, coverPath)}\n`, images };
}

function chapterToTypst(book: Book, index: number, paths: Map<string, string>): string {
  return `${preamble(book)}

#set page(numbering: "1")
#counter(page).update(1)

${openerCall(book, index)}

${chapterBody(book.chapters[index].content, paths)}
`;
}

export function chapterToPdfInputs(book: Book, index: number): { source: string; images: ImageInput[] } {
  const chapter = book.chapters[index];
  const { images, paths } = collectImages([chapter.content]);
  return { source: chapterToTypst(book, index, paths), images };
}
