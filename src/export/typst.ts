import type { JSONContent } from "@tiptap/core";
import { type Book, type FigurePlacement, type TrimSize, FIGURE_WIDTH, bodyNumber, chapterKind, isResizablePlacement, partNumber, partRoman } from "../model/book";
import { fontFamilyName, fontsUsed } from "../model/fonts";
import type { ImageInput } from "../ipc";

export interface PdfFonts {
  bundled: string[];
  system: string[];
}

function pdfFonts(book: Book): PdfFonts {
  const used = fontsUsed(book.settings.fonts);
  return { bundled: used.bundled.map((f) => f.id), system: used.system };
}

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

function localeParams(tag: string): string {
  const parts = (tag || "en").trim().split(/[-_]/);
  const primary = parts[0] ?? "";
  const lang = /^[a-zA-Z]{2,3}$/.test(primary) ? primary.toLowerCase() : "en";
  const region = parts.slice(1).find((p) => /^[a-zA-Z]{2}$/.test(p));
  const params = `lang: ${str(lang)}`;
  return region ? `${params}, region: ${str(region.toUpperCase())}` : params;
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

function aligned(align: unknown, body: string): string {
  if (align === "center" || align === "right") return `#align(${align})[#set par(justify: false)\n${body}]`;
  return body;
}

function block(node: JSONContent, paths: Map<string, string>): string {
  switch (node.type) {
    case "paragraph": {
      const body = inlines(node.content, true);
      if (!body.trim()) return "~";
      const align = node.attrs?.align;
      if (align === "center" || align === "right") return aligned(align, body);
      return node.attrs?.indent ? `#h(1.3em)${body}` : body;
    }
    case "heading":
      return aligned(node.attrs?.align, `#heading(level: ${node.attrs?.level ?? 2})[${inlines(node.content, true)}]`);
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
  const body = fontFamilyName(book.settings.fonts.body);
  const heading = fontFamilyName(book.settings.fonts.heading);
  return `#set document(title: ${str(meta.title || "Untitled")}, author: ${str(meta.author)})
#set page(
  width: ${trim.w} + ${bleed},
  height: ${trim.h} + ${bleed} * 2,
  margin: (inside: 0.875in, outside: 0.625in + ${bleed}, top: 0.8in + ${bleed}, bottom: 0.8in + ${bleed}),
  binding: left,
)
#set text(font: ${str(body)}, size: 11pt, ${localeParams(meta.language)}, hyphenate: true)
#set par(justify: true, leading: 0.72em, spacing: 0.72em)
#show heading: set text(font: ${str(heading)}, weight: "medium")
#show heading.where(level: 1): set text(size: 22pt)
#show heading.where(level: 2): set block(above: 1.4em, below: 0.6em)
#show heading.where(level: 3): set block(above: 1.2em, below: 0.5em)

#let scenebreak = align(center)[#v(0.5em) #line(length: 13%, stroke: 0.5pt + rgb("#d6cfbd")) #v(0.5em)]

#let blockquote(body) = pad(left: 1.2em)[#set text(style: "italic", fill: rgb("#6b6458")); #body]

#let titlepage(title, subtitle, author) = {
  v(2.4in)
  align(center)[
    #text(font: ${str(heading)}, size: 30pt, weight: "medium")[#title]
    #if subtitle != "" {
      v(0.6em)
      text(font: ${str(body)}, size: 15pt, style: "italic", fill: rgb("#6b6458"))[#subtitle]
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
  if not info.nomargin { v(2.1in) }
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

#let openpart(info) = {
  pagebreak(to: "odd", weak: true)
  [#metadata(info) <chap>]
  if not info.nomargin { v(3.1in) }
  align(center)[
    #text(font: "Hanken Grotesk", size: 10pt, weight: "semibold", tracking: 0.34em)[#upper("Part " + info.numeral)]
    #if not info.notitle and info.title != "" {
      v(0.85em)
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
    #heading(level: 1, numbering: none, outlined: false, bookmarked: true)[#text(font: ${str(heading)}, size: 24pt, weight: "medium")[Contents]]
  ]
  context {
    let items = query(<chap>).filter(it => it.value.toc != false)
    let prev = none
    for it in items {
      let info = it.value
      let pg = pageof(it.location())
      let body = info.kind == "body"
      let part = info.kind == "part"
      let gap = if prev != none and prev != info.kind { 1.7em } else { 0.95em }
      prev = info.kind
      block(width: 100%, above: gap, below: 0.95em, {
        set text(size: 11.5pt)
        link(it.location())[#grid(
          columns: (1.9em, 1fr, auto),
          column-gutter: (0.65em, 1em),
          align: (right + top, left + top, right + top),
          if body { info.num } else { [] },
          if part {
            strong(if info.title != "" { "Part " + info.numeral + ": " + info.title } else { "Part " + info.numeral })
          } else if body { info.title } else { emph(info.title) },
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
  const body = fontFamilyName(book.settings.fonts.body);
  const heading = fontFamilyName(book.settings.fonts.heading);

  if (c.kind === "image" && coverPath) {
    return `#page(margin: 0pt, numbering: none)[#image(${str(coverPath)}, width: 100%, height: 100%, fit: "cover")]`;
  }

  const subtitle = meta.subtitle
    ? `#v(0.55em)\n  #text(font: ${str(body)}, size: 14pt, style: "italic")[${esc(meta.subtitle)}]`
    : "";
  const author = meta.author
    ? `#v(1fr)\n  #text(font: "Hanken Grotesk", size: 11pt, weight: "semibold", tracking: 0.24em)[#upper[${esc(meta.author)}]]\n  #v(0.55in)`
    : "#v(1fr)";

  return `#page(margin: 0pt, numbering: none, fill: rgb(${str(c.bg)}))[
  #set align(center)
  #set text(fill: rgb(${str(c.ink)}))
  #v(1fr)
  #text(font: ${str(heading)}, size: 30pt, weight: "medium")[${esc(meta.title || "Untitled")}]
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

const SCRIPT_RANGES: { label: string; test: RegExp }[] = [
  { label: "CJK", test: /[぀-鿿가-힯豈-﫿]/u },
  { label: "Arabic", test: /[؀-ۿݐ-ݿ]/u },
  { label: "Hebrew", test: /[֐-׿]/u },
  { label: "Devanagari", test: /[ऀ-ॿ]/u },
  { label: "Thai", test: /[฀-๿]/u },
  { label: "emoji", test: /[☀-➿]|[\u{1f000}-\u{1faff}]/u },
];

function collectText(book: Book): string {
  const parts = [book.metadata.title, book.metadata.subtitle, book.metadata.author];
  const visit = (node: JSONContent) => {
    if (node.type === "text" && node.text) parts.push(node.text);
    if (node.attrs?.caption) parts.push(String(node.attrs.caption));
    (node.content ?? []).forEach(visit);
  };
  book.chapters.forEach((chapter) => {
    parts.push(chapter.title);
    visit(chapter.content);
  });
  return parts.join("\n");
}

export function unsupportedScripts(book: Book): string[] {
  const text = collectText(book);
  return SCRIPT_RANGES.filter((s) => s.test.test(text)).map((s) => s.label);
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
  const kind = chapterKind(chapter);
  if (kind === "part") return true;
  if (chapter.noTitle && kind !== "body") return false;
  const title = cleanTitle(chapter.title) || "Untitled";
  return !(kind === "front" && normTitle(title) === normTitle(book.metadata.title));
}

function openerCall(book: Book, index: number): string {
  const chapter = book.chapters[index];
  const kind = chapterKind(chapter);
  const notitle = !!chapter.noTitle;
  const nomargin = !!chapter.noMargin;
  if (kind === "part") {
    const numeral = partRoman(partNumber(book.chapters, index) ?? 0);
    const title = notitle ? "" : cleanTitle(chapter.title);
    return `#openpart((kind: "part", numeral: ${str(numeral)}, title: ${str(title)}, notitle: ${notitle}, nomargin: ${nomargin}, toc: ${inToc(book, index)}))`;
  }
  const num = kind === "body" ? String(bodyNumber(book.chapters, index)) : "";
  const title = notitle ? "" : cleanTitle(chapter.title) || "Untitled";
  return `#openchapter((kind: ${str(kind)}, num: ${str(num)}, title: ${str(title)}, notitle: ${notitle}, nomargin: ${nomargin}, toc: ${inToc(book, index)}))`;
}

export function bookToTypst(book: Book, paths: Map<string, string> = new Map(), coverPath?: string): string {
  const meta = book.metadata;
  const cover = `${coverBlock(book, coverPath)}\n#pagebreak(weak: true)`;
  const front = `#set page(numbering: none)
#titlepage(${str(meta.title || "Untitled")}, ${str(meta.subtitle)}, ${str(meta.author)})
#pagebreak(weak: true)
#set page(numbering: "i")
#counter(page).update(1)`;

  const firstMain = book.chapters.findIndex((c) => chapterKind(c) === "body" || chapterKind(c) === "part");
  const firstToc = book.chapters.findIndex((_, i) => inToc(book, i));

  const body = book.chapters
    .map((chapter, i) => {
      const toc = i === firstToc ? `#contents()\n\n` : "";
      const reset = i === firstMain ? `#set page(numbering: "1")\n#counter(page).update(1)\n` : "";
      return `${toc}${reset}${openerCall(book, i)}\n\n${chapterBody(chapter.content, paths)}`;
    })
    .join("\n\n");

  return `${preamble(book)}\n\n${cover}\n\n${front}\n\n${body}\n`;
}

export function bookToPdfInputs(book: Book): { source: string; images: ImageInput[]; fonts: PdfFonts } {
  const { images, paths } = extractImages(book);
  const coverPath = pushCoverImage(book, images);
  return { source: bookToTypst(book, paths, coverPath), images, fonts: pdfFonts(book) };
}

export function coverToPdfInputs(book: Book): { source: string; images: ImageInput[]; fonts: PdfFonts } {
  const images: ImageInput[] = [];
  const coverPath = pushCoverImage(book, images);
  return { source: `${preamble(book)}\n\n${coverBlock(book, coverPath)}\n`, images, fonts: pdfFonts(book) };
}

function chapterToTypst(book: Book, index: number, paths: Map<string, string>): string {
  return `${preamble(book)}

#set page(numbering: "1")
#counter(page).update(1)

${openerCall(book, index)}

${chapterBody(book.chapters[index].content, paths)}
`;
}

export function chapterToPdfInputs(book: Book, index: number): { source: string; images: ImageInput[]; fonts: PdfFonts } {
  const chapter = book.chapters[index];
  const { images, paths } = collectImages([chapter.content]);
  return { source: chapterToTypst(book, index, paths), images, fonts: pdfFonts(book) };
}
