import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { JSONContent } from "@tiptap/core";
import { type Book, type BookMetadata, type Chapter, type Cover, createCover } from "../model/book";

export interface RawFile {
  path: string;
  data: string;
  encoding: "utf8" | "base64";
}

type Mark = { type: string; attrs?: Record<string, unknown> };

const BLOCK_TAGS = new Set([
  "p", "div", "section", "article", "main", "header", "footer", "aside",
  "blockquote", "ul", "ol", "li", "figure", "img", "hr", "table", "tr",
  "td", "th", "thead", "tbody", "h1", "h2", "h3", "h4", "h5", "h6",
]);

function normalize(path: string): string {
  return path.replace(/^\.?\//, "");
}

function dirOf(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "" : path.slice(0, i);
}

function baseName(path: string): string {
  const file = path.split(/[\\/]/).pop() ?? path;
  return file.replace(/\.[^.]+$/, "");
}

function decodeHref(href: string): string {
  const clean = href.split("#")[0];
  try {
    return decodeURIComponent(clean);
  } catch {
    return clean;
  }
}

function resolvePath(baseDir: string, rel: string): string {
  if (rel.startsWith("/")) return normalize(rel);
  const parts = (baseDir ? baseDir.split("/") : []).concat(rel.split("/"));
  const out: string[] = [];
  for (const part of parts) {
    if (part === "" || part === ".") continue;
    if (part === "..") out.pop();
    else out.push(part);
  }
  return out.join("/");
}

function mimeFromExt(path: string): string {
  const ext = path.toLowerCase().split(".").pop() ?? "";
  if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
  if (ext === "png") return "image/png";
  if (ext === "gif") return "image/gif";
  if (ext === "webp") return "image/webp";
  if (ext === "svg") return "image/svg+xml";
  return "application/octet-stream";
}

function parseXml(text: string, mime: DOMParserSupportedType): Document {
  return new DOMParser().parseFromString(text, mime);
}

function firstByLocal(root: Document | Element, local: string): Element | null {
  for (const el of Array.from(root.getElementsByTagName("*"))) {
    if (el.localName === local) return el;
  }
  return null;
}

function localText(root: Document | Element, local: string): string {
  return firstByLocal(root, local)?.textContent?.trim() ?? "";
}

function hasMark(marks: Mark[], type: string): boolean {
  return marks.some((m) => m.type === type);
}

function addMark(marks: Mark[], type: string, attrs?: Record<string, unknown>): Mark[] {
  return hasMark(marks, type) ? marks : [...marks, attrs ? { type, attrs } : { type }];
}

function collectInline(node: Node, marks: Mark[], out: JSONContent[]): void {
  node.childNodes.forEach((child) => {
    if (child.nodeType === 3) {
      const text = (child.nodeValue ?? "").replace(/\s+/g, " ");
      if (text === "") return;
      out.push(marks.length ? { type: "text", text, marks } : { type: "text", text });
      return;
    }
    if (child.nodeType !== 1) return;
    const el = child as Element;
    const tag = el.localName.toLowerCase();
    if (tag === "br") {
      out.push({ type: "hardBreak" });
    } else if (tag === "strong" || tag === "b") {
      collectInline(el, addMark(marks, "bold"), out);
    } else if (tag === "em" || tag === "i") {
      collectInline(el, addMark(marks, "italic"), out);
    } else if (tag === "u") {
      collectInline(el, addMark(marks, "underline"), out);
    } else if (tag === "s" || tag === "strike" || tag === "del") {
      collectInline(el, addMark(marks, "strike"), out);
    } else if (tag === "code") {
      collectInline(el, addMark(marks, "code"), out);
    } else if (tag === "a") {
      const href = el.getAttribute("href");
      collectInline(el, href ? addMark(marks, "link", { href }) : marks, out);
    } else if (!BLOCK_TAGS.has(tag)) {
      collectInline(el, marks, out);
    }
  });
}

function inlineOf(el: Element): JSONContent[] {
  const out: JSONContent[] = [];
  collectInline(el, [], out);
  return trimInline(out);
}

function trimInline(nodes: JSONContent[]): JSONContent[] {
  const out = nodes.slice();
  while (out.length && out[0].type === "text") {
    out[0] = { ...out[0], text: (out[0].text ?? "").replace(/^\s+/, "") };
    if (out[0].text === "") out.shift();
    else break;
  }
  while (out.length) {
    const last = out[out.length - 1];
    if (last.type !== "text") break;
    out[out.length - 1] = { ...last, text: (last.text ?? "").replace(/\s+$/, "") };
    if (out[out.length - 1].text === "") out.pop();
    else break;
  }
  return out.filter((n) => n.type !== "text" || n.text !== "");
}

function meaningful(inline: JSONContent[]): boolean {
  return inline.some((n) => (n.type === "text" && n.text!.trim() !== "") || n.type === "hardBreak");
}

function imgFigure(img: Element, caption = ""): JSONContent[] {
  const src = img.getAttribute("src");
  if (src && src.startsWith("data:")) {
    return [
      {
        type: "figure",
        attrs: { src, alt: img.getAttribute("alt") ?? "", caption, placement: "full-width" },
      },
    ];
  }
  const alt = (img.getAttribute("alt") ?? "").trim();
  return alt ? [{ type: "paragraph", content: [{ type: "text", text: alt }] }] : [];
}

function svgFigure(el: Element): JSONContent[] {
  const markup = el.outerHTML;
  if (!markup) return [];
  const src = `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(markup)))}`;
  return [{ type: "figure", attrs: { src, alt: "", caption: "", placement: "full-width" } }];
}

function figureFrom(el: Element): JSONContent[] {
  const img = el.querySelector("img");
  if (!img) return [];
  const caption = el.querySelector("figcaption")?.textContent?.trim() ?? "";
  return imgFigure(img, caption);
}

function paragraphAndFigures(el: Element): JSONContent[] {
  const out: JSONContent[] = [];
  const inline = inlineOf(el);
  if (meaningful(inline)) out.push({ type: "paragraph", content: inline });
  el.querySelectorAll("img").forEach((img) => out.push(...imgFigure(img)));
  return out;
}

function hasBlockChild(el: Element): boolean {
  return Array.from(el.children).some((c) => BLOCK_TAGS.has(c.localName.toLowerCase()));
}

function listItems(el: Element, depth: number): JSONContent[] {
  const items: JSONContent[] = [];
  Array.from(el.children)
    .filter((c) => c.localName.toLowerCase() === "li")
    .forEach((li) => {
      const content = blocksFrom(li, depth + 1);
      items.push({ type: "listItem", content: content.length ? content : [{ type: "paragraph" }] });
    });
  return items;
}

function tableBlocks(el: Element): JSONContent[] {
  const out: JSONContent[] = [];
  Array.from(el.querySelectorAll("tr")).forEach((tr) => {
    const content: JSONContent[] = [];
    Array.from(tr.children)
      .filter((c) => c.localName.toLowerCase() === "td" || c.localName.toLowerCase() === "th")
      .forEach((cell) => {
        const inline = inlineOf(cell);
        if (!meaningful(inline)) return;
        if (content.length) content.push({ type: "text", text: "  ·  " });
        content.push(...inline);
      });
    if (meaningful(content)) out.push({ type: "paragraph", content });
  });
  return out;
}

function blockFromElement(el: Element, depth: number): JSONContent[] {
  const tag = el.localName.toLowerCase();
  switch (tag) {
    case "p":
      return paragraphAndFigures(el);
    case "h1":
    case "h2":
    case "h3":
    case "h4":
    case "h5":
    case "h6": {
      const level = tag === "h1" || tag === "h2" ? 2 : 3;
      const content = inlineOf(el);
      return meaningful(content) ? [{ type: "heading", attrs: { level }, content }] : [];
    }
    case "blockquote": {
      const inner = blocksFrom(el, depth + 1);
      return [{ type: "blockquote", content: inner.length ? inner : [{ type: "paragraph" }] }];
    }
    case "ul": {
      const items = listItems(el, depth);
      return items.length ? [{ type: "bulletList", content: items }] : [];
    }
    case "ol": {
      const items = listItems(el, depth);
      return items.length ? [{ type: "orderedList", content: items }] : [];
    }
    case "hr":
      return [{ type: "horizontalRule" }];
    case "figure":
      return figureFrom(el);
    case "img":
      return imgFigure(el);
    case "svg":
      return svgFigure(el);
    case "table":
      return tableBlocks(el);
    case "script":
    case "style":
      return [];
    default:
      return hasBlockChild(el) ? blocksFrom(el, depth + 1) : paragraphAndFigures(el);
  }
}

function blocksFrom(parent: Node, depth = 0): JSONContent[] {
  if (depth > 64) return [];
  const out: JSONContent[] = [];
  parent.childNodes.forEach((child) => {
    if (child.nodeType === 3) {
      const text = (child.nodeValue ?? "").replace(/\s+/g, " ").trim();
      if (text !== "") out.push({ type: "paragraph", content: [{ type: "text", text }] });
    } else if (child.nodeType === 1) {
      out.push(...blockFromElement(child as Element, depth));
    }
  });
  return out;
}

function headingText(node: JSONContent): string {
  return (node.content ?? []).map((n) => n.text ?? "").join("").trim();
}

function isbnChecksum(s: string): boolean {
  if (s.length === 10) {
    let sum = 0;
    for (let i = 0; i < 10; i++) {
      const v = s[i] === "X" ? 10 : Number(s[i]);
      if (Number.isNaN(v) || (s[i] === "X" && i !== 9)) return false;
      sum += v * (10 - i);
    }
    return sum % 11 === 0;
  }
  if (s.length === 13 && /^\d{13}$/.test(s)) {
    let sum = 0;
    for (let i = 0; i < 13; i++) sum += Number(s[i]) * (i % 2 === 0 ? 1 : 3);
    return sum % 10 === 0;
  }
  return false;
}

function findIsbn(opf: Document): string {
  const ids = Array.from(opf.getElementsByTagName("*")).filter((el) => el.localName === "identifier");
  const ranked = ids
    .map((el) => ({
      text: el.textContent ?? "",
      tagged:
        /isbn/i.test(el.getAttribute("opf:scheme") || el.getAttribute("scheme") || "") ||
        /urn:isbn:/i.test(el.textContent ?? ""),
    }))
    .sort((a, b) => Number(b.tagged) - Number(a.tagged));
  for (const { text } of ranked) {
    const cleaned = text.replace(/[^0-9Xx]/g, "").toUpperCase();
    if (isbnChecksum(cleaned)) return cleaned;
  }
  return "";
}

interface ManifestItem {
  href: string;
  mediaType: string;
  properties: string;
}

function readManifest(opf: Document, opfDir: string): Map<string, ManifestItem> {
  const items = new Map<string, ManifestItem>();
  Array.from(opf.getElementsByTagName("*"))
    .filter((el) => el.localName === "item")
    .forEach((el) => {
      const id = el.getAttribute("id");
      const href = el.getAttribute("href");
      if (!id || !href) return;
      items.set(id, {
        href: resolvePath(opfDir, decodeHref(href)),
        mediaType: el.getAttribute("media-type") ?? "",
        properties: el.getAttribute("properties") ?? "",
      });
    });
  return items;
}

function readSpine(opf: Document, manifest: Map<string, ManifestItem>): ManifestItem[] {
  return Array.from(opf.getElementsByTagName("*"))
    .filter((el) => el.localName === "itemref")
    .map((el) => manifest.get(el.getAttribute("idref") ?? ""))
    .filter((item): item is ManifestItem => !!item)
    .filter((item) => !item.properties.split(/\s+/).includes("nav"))
    .filter((item) => /xhtml|html/.test(item.mediaType) || /\.x?html?$/.test(item.href));
}

function cleanLabel(text: string | null | undefined): string {
  return (text ?? "")
    .replace(/[\u0000-\u001f\u007f-\u009f\u2028\u2029]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function readNavTitles(
  manifest: Map<string, ManifestItem>,
  byPath: Map<string, RawFile>,
): Map<string, string> {
  const titles = new Map<string, string>();
  const nav = Array.from(manifest.values()).find((m) =>
    m.properties.split(/\s+/).includes("nav"),
  );
  const ncx = Array.from(manifest.values()).find((m) =>
    m.mediaType.includes("dtbncx"),
  );

  if (nav) {
    const file = byPath.get(normalize(nav.href));
    if (file) {
      const doc = parseXml(file.data, "text/html");
      const baseDir = dirOf(nav.href);
      doc.querySelectorAll("a[href]").forEach((a) => {
        const key = resolvePath(baseDir, decodeHref(a.getAttribute("href")!));
        const label = cleanLabel(a.textContent);
        if (label && !titles.has(key)) titles.set(key, label);
      });
    }
  }
  if (!titles.size && ncx) {
    const file = byPath.get(normalize(ncx.href));
    if (file) {
      const doc = parseXml(file.data, "application/xml");
      const baseDir = dirOf(ncx.href);
      Array.from(doc.getElementsByTagName("*"))
        .filter((el) => el.localName === "navPoint")
        .forEach((point) => {
          const label = cleanLabel(firstByLocal(point, "text")?.textContent);
          const src = firstByLocal(point, "content")?.getAttribute("src");
          if (label && src) {
            const key = resolvePath(baseDir, decodeHref(src));
            if (!titles.has(key)) titles.set(key, label);
          }
        });
    }
  }
  return titles;
}

function fileToDataUri(file: RawFile, mime: string): string {
  const data = file.encoding === "base64" ? file.data : btoa(unescape(encodeURIComponent(file.data)));
  return `data:${mime};base64,${data}`;
}

function resolveImages(
  doc: Document,
  baseDir: string,
  byPath: Map<string, RawFile>,
  manifest: Map<string, ManifestItem>,
): void {
  const mimeByPath = new Map<string, string>();
  manifest.forEach((item) => mimeByPath.set(item.href, item.mediaType));

  doc.querySelectorAll("img").forEach((img) => {
    const src = img.getAttribute("src");
    if (!src || src.startsWith("data:")) return;
    const resolved = resolvePath(baseDir, decodeHref(src));
    const file = byPath.get(normalize(resolved));
    if (!file) {
      img.removeAttribute("src");
      return;
    }
    img.setAttribute("src", fileToDataUri(file, mimeByPath.get(resolved) || mimeFromExt(resolved)));
  });
}

function findCover(
  opf: Document,
  manifest: Map<string, ManifestItem>,
  byPath: Map<string, RawFile>,
): Cover {
  let item = Array.from(manifest.values()).find((m) => m.properties.split(/\s+/).includes("cover-image"));
  if (!item) {
    const metaCover = Array.from(opf.getElementsByTagName("*")).find(
      (el) => el.localName === "meta" && el.getAttribute("name") === "cover",
    );
    const id = metaCover?.getAttribute("content");
    if (id) item = manifest.get(id);
  }
  if (!item) return createCover();
  const file = byPath.get(normalize(item.href));
  if (!file) return createCover();
  return { ...createCover(), kind: "image", image: fileToDataUri(file, item.mediaType || mimeFromExt(item.href)) };
}

function buildChapter(
  file: RawFile,
  baseDir: string,
  navTitle: string | undefined,
  index: number,
  byPath: Map<string, RawFile>,
  manifest: Map<string, ManifestItem>,
): Chapter {
  const doc = parseXml(file.data, "text/html");
  resolveImages(doc, baseDir, byPath, manifest);
  const blocks = blocksFrom(doc.body);

  let title = navTitle ?? "";
  if (blocks.length && blocks[0].type === "heading") {
    const text = headingText(blocks[0]);
    if (!title) title = text;
    if (text && text.toLowerCase() === title.toLowerCase()) blocks.shift();
  }

  return {
    id: crypto.randomUUID(),
    title: title || `Chapter ${index + 1}`,
    content: { type: "doc", content: blocks.length ? blocks : [{ type: "paragraph" }] },
    updatedAt: Date.now(),
  };
}

export function filesToBook(files: RawFile[], fallbackName = "Imported book"): Book {
  const byPath = new Map<string, RawFile>();
  files.forEach((f) => byPath.set(normalize(f.path), f));

  const container = byPath.get("META-INF/container.xml");
  let opfPath = "";
  if (container) {
    const doc = parseXml(container.data, "application/xml");
    opfPath = firstByLocal(doc, "rootfile")?.getAttribute("full-path") ?? "";
  }
  if (!opfPath) {
    opfPath = files.find((f) => f.path.toLowerCase().endsWith(".opf"))?.path ?? "";
  }
  const opfFile = opfPath ? byPath.get(normalize(opfPath)) : undefined;
  if (!opfFile) throw new Error("Not a valid EPUB: no package document found.");

  const opf = parseXml(opfFile.data, "application/xml");
  const opfDir = dirOf(normalize(opfPath));
  const manifest = readManifest(opf, opfDir);
  const spine = readSpine(opf, manifest);
  const navTitles = readNavTitles(manifest, byPath);

  const chapters: Chapter[] = [];
  spine.forEach((item, i) => {
    const file = byPath.get(normalize(item.href));
    if (!file) return;
    try {
      chapters.push(
        buildChapter(file, dirOf(item.href), navTitles.get(item.href), i, byPath, manifest),
      );
    } catch {
      return;
    }
  });
  if (!chapters.length) throw new Error("Not a valid EPUB: no readable chapters found.");

  const metadata: BookMetadata = {
    title: localText(opf, "title") || fallbackName,
    subtitle: "",
    author: localText(opf, "creator"),
    isbn: findIsbn(opf),
    language: localText(opf, "language") || "en",
  };

  return {
    schema: "margin/1",
    id: crypto.randomUUID(),
    metadata,
    theme: "quiet-press",
    settings: { trim: "6x9", bleed: true },
    cover: findCover(opf, manifest, byPath),
    chapters,
  };
}

export async function importEpub(): Promise<Book | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "EPUB", extensions: ["epub"] }],
  });
  if (typeof selected !== "string") return null;
  const files = await invoke<RawFile[]>("unzip_epub", { path: selected });
  return filesToBook(files, baseName(selected));
}
