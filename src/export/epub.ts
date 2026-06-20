import type { JSONContent } from "@tiptap/core";
import { invoke } from "@tauri-apps/api/core";
import { type Book, TRIM_DIMS, bodyNumber, chapterKind } from "../model/book";

export interface EpubFile {
  path: string;
  data: string;
  encoding: "utf8" | "base64";
}

function esc(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function attr(text: string): string {
  return esc(text).replace(/"/g, "&quot;");
}

function slug(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function imageExtension(dataUrl: string): string {
  const match = /^data:image\/([a-z0-9.+-]+)/i.exec(dataUrl);
  const kind = (match?.[1] ?? "png").toLowerCase();
  return kind === "jpeg" ? "jpg" : kind;
}

function mediaType(path: string): string {
  if (path.endsWith(".jpg")) return "image/jpeg";
  if (path.endsWith(".png")) return "image/png";
  if (path.endsWith(".gif")) return "image/gif";
  if (path.endsWith(".webp")) return "image/webp";
  if (path.endsWith(".svg")) return "image/svg+xml";
  return "application/octet-stream";
}

function inline(node: JSONContent): string {
  if (node.type === "text") {
    let text = esc(node.text ?? "");
    const marks = node.marks ?? [];
    if (marks.some((m) => m.type === "bold")) text = `<strong>${text}</strong>`;
    if (marks.some((m) => m.type === "italic")) text = `<em>${text}</em>`;
    const href = marks.find((m) => m.type === "link")?.attrs?.href;
    if (href) text = `<a href="${attr(href)}">${text}</a>`;
    return text;
  }
  if (node.type === "hardBreak") return "<br/>";
  return "";
}

function inlines(content: JSONContent[] = []): string {
  return content.map(inline).join("");
}

function listItem(item: JSONContent): string {
  const inner = (item.content ?? [])
    .filter((c) => c.type === "paragraph")
    .map((p) => inlines(p.content))
    .join(" ");
  return `<li>${inner}</li>`;
}

function figure(node: JSONContent, paths: Map<string, string>): string {
  const path = paths.get(node.attrs?.src);
  if (!path) return "";
  const placement = node.attrs?.placement ?? "inline";
  const alt = attr(node.attrs?.alt ?? "");
  const caption = node.attrs?.caption
    ? `<figcaption>${esc(node.attrs.caption)}</figcaption>`
    : "";
  return `<figure class="placement-${attr(placement)}"><img src="${attr(path)}" alt="${alt}"/>${caption}</figure>`;
}

function block(node: JSONContent, paths: Map<string, string>): string {
  switch (node.type) {
    case "paragraph":
      return `<p>${inlines(node.content)}</p>`;
    case "heading":
      return node.attrs?.level === 3
        ? `<h3>${inlines(node.content)}</h3>`
        : `<h2>${inlines(node.content)}</h2>`;
    case "blockquote":
      return `<blockquote>${(node.content ?? []).map((n) => block(n, paths)).join("")}</blockquote>`;
    case "bulletList":
      return `<ul>${(node.content ?? []).map(listItem).join("")}</ul>`;
    case "orderedList":
      return `<ol>${(node.content ?? []).map(listItem).join("")}</ol>`;
    case "horizontalRule":
      return `<hr class="scene-break"/>`;
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
    .join("\n");
}

function wrapTitle(title: string, max = 16): string[] {
  const words = title.split(/\s+/).filter(Boolean);
  const lines: string[] = [];
  let line = "";
  for (const word of words) {
    if (line && (line + " " + word).length > max) {
      lines.push(line);
      line = word;
    } else {
      line = line ? line + " " + word : word;
    }
  }
  if (line) lines.push(line);
  return lines.length ? lines.slice(0, 4) : ["Untitled"];
}

function coverSvg(book: Book): string {
  const c = book.cover;
  const m = book.metadata;
  const dims = TRIM_DIMS[book.settings.trim];
  const W = 1200;
  const H = Math.round((W * dims.h) / dims.w);

  const lines = wrapTitle(m.title || "Untitled");
  const titleSize = lines.length > 2 ? 88 : 104;
  const lineH = titleSize * 1.12;
  const blockMid = H * 0.4;
  const firstY = Math.round(blockMid - ((lines.length - 1) * lineH) / 2);
  const titleSpans = lines
    .map((l, i) => `<tspan x="${W / 2}" y="${firstY + Math.round(i * lineH)}">${esc(l)}</tspan>`)
    .join("");
  const ruleY = firstY + Math.round((lines.length - 1) * lineH + 84);
  const rule = `<line x1="${W / 2 - 96}" y1="${ruleY}" x2="${W / 2 + 96}" y2="${ruleY}" stroke="${c.ink}" stroke-width="3" opacity="0.7"/>`;
  const subtitle = m.subtitle
    ? `<text x="${W / 2}" y="${ruleY + 96}" font-family="Georgia, 'Times New Roman', serif" font-style="italic" font-size="50" fill="${c.ink}" text-anchor="middle" opacity="0.85">${esc(m.subtitle)}</text>`
    : "";
  const author = m.author
    ? `<text x="${W / 2}" y="${H - 132}" font-family="'Helvetica Neue', Arial, sans-serif" font-size="40" font-weight="600" letter-spacing="9" fill="${c.ink}" text-anchor="middle" opacity="0.92">${esc(m.author.toUpperCase())}</text>`
    : "";

  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">
  <rect width="${W}" height="${H}" fill="${c.bg}"/>
  <text font-family="Georgia, 'Times New Roman', serif" font-size="${titleSize}" font-weight="500" fill="${c.ink}" text-anchor="middle">${titleSpans}</text>
  ${rule}
  ${subtitle}
  ${author}
</svg>`;
}

function buildCover(book: Book): { asset: EpubFile; href: string; mediaType: string } {
  const c = book.cover;
  if (c.kind === "image" && c.image.startsWith("data:")) {
    const href = `assets/cover.${imageExtension(c.image)}`;
    return {
      asset: { path: `OEBPS/${href}`, data: c.image.slice(c.image.indexOf(",") + 1), encoding: "base64" },
      href,
      mediaType: mediaType(href),
    };
  }
  const href = "assets/cover.svg";
  return {
    asset: { path: `OEBPS/${href}`, data: coverSvg(book), encoding: "utf8" },
    href,
    mediaType: "image/svg+xml",
  };
}

function coverXhtml(book: Book, href: string): string {
  const title = book.metadata.title || "Cover";
  return `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="${attr(book.metadata.language || "en")}">
  <head>
    <title>${esc(title)}</title>
    <link rel="stylesheet" type="text/css" href="style.css"/>
  </head>
  <body epub:type="cover">
    <section class="cover"><img src="${attr(href)}" alt="${attr(title)}"/></section>
  </body>
</html>`;
}

function extractFigures(book: Book): { assets: EpubFile[]; paths: Map<string, string> } {
  const paths = new Map<string, string>();
  const assets: EpubFile[] = [];

  const visit = (node: JSONContent) => {
    if (node.type === "figure") {
      const src: string | undefined = node.attrs?.src;
      if (src && src.startsWith("data:") && !paths.has(src)) {
        const path = `assets/figure-${assets.length + 1}.${imageExtension(src)}`;
        paths.set(src, path);
        assets.push({
          path: `OEBPS/${path}`,
          data: src.slice(src.indexOf(",") + 1),
          encoding: "base64",
        });
      }
    }
    (node.content ?? []).forEach(visit);
  };

  book.chapters.forEach((chapter) => visit(chapter.content));
  return { assets, paths };
}

function identifier(book: Book): string {
  if (book.metadata.isbn) return book.metadata.isbn;
  const seed = slug(book.metadata.title || "untitled") || "untitled";
  return `urn:uuid:margin-${seed}`;
}

function containerXml(): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>`;
}

function contentOpf(
  book: Book,
  chapterIds: string[],
  assetPaths: string[],
  cover: { href: string; mediaType: string },
): string {
  const meta = book.metadata;
  const manifest: string[] = [
    `<item id="cover-image" href="${attr(cover.href)}" media-type="${cover.mediaType}" properties="cover-image"/>`,
    `<item id="cover" href="cover.xhtml" media-type="application/xhtml+xml"/>`,
    `<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>`,
    `<item id="css" href="style.css" media-type="text/css"/>`,
    `<item id="font-regular" href="fonts/Literata-VF.ttf" media-type="font/ttf"/>`,
    `<item id="font-italic" href="fonts/Literata-Italic-VF.ttf" media-type="font/ttf"/>`,
  ];
  chapterIds.forEach((_, i) => {
    manifest.push(
      `<item id="chapter-${i + 1}" href="chapter-${i + 1}.xhtml" media-type="application/xhtml+xml"/>`,
    );
  });
  assetPaths.forEach((path, i) => {
    manifest.push(
      `<item id="asset-${i + 1}" href="${attr(path)}" media-type="${mediaType(path)}"/>`,
    );
  });

  const spine = [`<itemref idref="cover" linear="yes"/>`, `<itemref idref="nav" linear="no"/>`]
    .concat(chapterIds.map((_, i) => `<itemref idref="chapter-${i + 1}"/>`))
    .join("\n    ");

  return `<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="book-id" xml:lang="${attr(meta.language || "en")}">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="book-id">${esc(identifier(book))}</dc:identifier>
    <dc:title>${esc(meta.title || "Untitled")}</dc:title>
    <dc:creator>${esc(meta.author || "")}</dc:creator>
    <dc:language>${esc(meta.language || "en")}</dc:language>
    <meta name="cover" content="cover-image"/>
    <meta property="dcterms:modified">2026-01-01T00:00:00Z</meta>
  </metadata>
  <manifest>
    ${manifest.join("\n    ")}
  </manifest>
  <spine>
    ${spine}
  </spine>
</package>`;
}

function navXhtml(book: Book): string {
  const items = book.chapters
    .map((chapter, i) => {
      const fallback = chapterKind(chapter) === "body" ? `Chapter ${bodyNumber(book.chapters, i)}` : "Untitled";
      return `      <li><a href="chapter-${i + 1}.xhtml">${esc(chapter.title || fallback)}</a></li>`;
    })
    .join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="${attr(book.metadata.language || "en")}">
  <head>
    <title>${esc(book.metadata.title || "Untitled")}</title>
    <link rel="stylesheet" type="text/css" href="style.css"/>
  </head>
  <body>
    <nav epub:type="toc" id="toc">
      <h1>Contents</h1>
      <ol>
${items}
      </ol>
    </nav>
  </body>
</html>`;
}

function chapterXhtml(book: Book, index: number, paths: Map<string, string>): string {
  const chapter = book.chapters[index];
  const kind = chapterKind(chapter);
  const num = bodyNumber(book.chapters, index);
  const title = chapter.title || (kind === "body" ? `Chapter ${num}` : "Untitled");
  const eyebrow = kind === "body" ? `<p class="eyebrow">Chapter ${num}</p>` : "";
  return `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="${attr(book.metadata.language || "en")}">
  <head>
    <title>${esc(title)}</title>
    <link rel="stylesheet" type="text/css" href="style.css"/>
  </head>
  <body epub:type="${kind === "body" ? "chapter" : kind === "front" ? "frontmatter" : "backmatter"}">
    <header class="chapter-opener">
      ${eyebrow}
      <h1>${esc(title)}</h1>
    </header>
    ${chapterBody(chapter.content, paths)}
  </body>
</html>`;
}

function styleCss(): string {
  return `@font-face {
  font-family: "Literata";
  font-style: normal;
  font-weight: 300 700;
  src: url("fonts/Literata-VF.ttf");
}

@font-face {
  font-family: "Literata";
  font-style: italic;
  font-weight: 300 700;
  src: url("fonts/Literata-Italic-VF.ttf");
}

html {
  font-family: "Literata", Georgia, serif;
}

.cover {
  margin: 0;
  padding: 0;
  text-align: center;
  page-break-after: always;
}

.cover img {
  max-width: 100%;
  max-height: 100vh;
}

body {
  margin: 0 5%;
  line-height: 1.5;
}

p {
  margin: 0;
  text-align: justify;
  text-indent: 1.3em;
  hyphens: auto;
  -webkit-hyphens: auto;
  -epub-hyphens: auto;
}

h1, h2, h3 {
  text-indent: 0;
  hyphens: none;
  line-height: 1.25;
  font-weight: 500;
}

h2, h3 {
  margin: 1.4em 0 0.6em;
}

p + p {
  text-indent: 1.3em;
}

.chapter-opener {
  text-align: center;
  margin: 3em 0 2.2em;
}

.chapter-opener p,
header + p,
h2 + p,
h3 + p,
blockquote + p,
figure + p,
hr + p {
  text-indent: 0;
}

.eyebrow {
  text-transform: uppercase;
  letter-spacing: 0.28em;
  font-size: 0.72em;
  margin: 0 0 0.7em;
}

.chapter-opener h1 {
  font-size: 1.7em;
  margin: 0;
}

blockquote {
  margin: 1.2em 1.6em;
  font-style: italic;
}

blockquote p {
  text-indent: 0;
  text-align: left;
}

ul, ol {
  margin: 1em 0;
  padding-left: 1.6em;
}

li {
  text-align: left;
}

hr.scene-break {
  border: 0;
  margin: 1.6em 0;
  text-align: center;
}

hr.scene-break::before {
  content: "\\2042";
  letter-spacing: 0.4em;
}

figure {
  margin: 1.4em 0;
  text-align: center;
}

figure img {
  max-width: 100%;
}

figcaption {
  font-size: 0.82em;
  font-style: italic;
  text-indent: 0;
  margin-top: 0.5em;
}

figure.placement-full-width img,
figure.placement-full-page img {
  width: 100%;
}

figure.placement-full-page {
  margin: 0;
  page-break-before: always;
  page-break-after: always;
}

figure.placement-float-top {
  float: top;
}
`;
}

export function bookToEpub(book: Book): EpubFile[] {
  const { assets, paths } = extractFigures(book);
  const cover = buildCover(book);
  const chapterIds = book.chapters.map((c) => c.id);
  const assetPaths = Array.from(paths.values());

  const files: EpubFile[] = [
    { path: "mimetype", data: "application/epub+zip", encoding: "utf8" },
    { path: "META-INF/container.xml", data: containerXml(), encoding: "utf8" },
    {
      path: "OEBPS/content.opf",
      data: contentOpf(book, chapterIds, assetPaths, cover),
      encoding: "utf8",
    },
    { path: "OEBPS/cover.xhtml", data: coverXhtml(book, cover.href), encoding: "utf8" },
    { path: "OEBPS/nav.xhtml", data: navXhtml(book), encoding: "utf8" },
    { path: "OEBPS/style.css", data: styleCss(), encoding: "utf8" },
    cover.asset,
  ];

  book.chapters.forEach((_, i) => {
    files.push({
      path: `OEBPS/chapter-${i + 1}.xhtml`,
      data: chapterXhtml(book, i, paths),
      encoding: "utf8",
    });
  });

  files.push(...assets);
  return files;
}

export async function buildEpub(book: Book): Promise<Uint8Array> {
  const files = bookToEpub(book);
  const buf = await invoke<ArrayBuffer>("package_epub", { files });
  return new Uint8Array(buf);
}
