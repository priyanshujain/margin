import { useEffect, useMemo, useRef, useState } from "react";
import { generateHTML } from "@tiptap/core";
import { useBook } from "../store/useBook";
import type { TrimSize } from "../model/book";
import { editorExtensions } from "../editor/extensions";
import { bookToPdfInputs } from "../export/typst";
import { compilePdf, isDesktop } from "../ipc";
import { PdfPreview } from "./PdfPreview";

const TRIMS: { value: TrimSize; label: string }[] = [
  { value: "6x9", label: "6 × 9 in" },
  { value: "5.5x8.5", label: "5.5 × 8.5 in" },
  { value: "5x8", label: "5 × 8 in" },
  { value: "a5", label: "A5" },
];

function TrimSelect() {
  const trim = useBook((s) => s.book?.settings.trim ?? "6x9");
  const setSettings = useBook((s) => s.setSettings);
  return (
    <select
      className="trim-select"
      value={trim}
      onChange={(e) => setSettings({ trim: e.target.value as TrimSize })}
      title="Trim size"
    >
      {TRIMS.map((t) => (
        <option key={t.value} value={t.value}>
          {t.label}
        </option>
      ))}
    </select>
  );
}

export function Dock() {
  return isDesktop ? <PdfDock /> : <HtmlDock />;
}

function PdfDock() {
  const book = useBook((s) => s.book);
  const [pdf, setPdf] = useState<Uint8Array | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    if (!book) return;
    clearTimeout(timer.current);
    setBusy(true);
    timer.current = setTimeout(() => {
      const { source, images } = bookToPdfInputs(book);
      compilePdf(source, images)
        .then((bytes) => {
          setPdf(bytes);
          setError(null);
        })
        .catch((e) => setError(String(e)))
        .finally(() => setBusy(false));
    }, 350);
    return () => clearTimeout(timer.current);
  }, [book]);

  return (
    <section className="dock">
      <div className="dock-head">
        <span className="label">Preview</span>
        <div className="dock-tools">
          {busy && <span className="meta">updating…</span>}
          <TrimSelect />
        </div>
      </div>
      {error ? (
        <pre className="dock-error">{error}</pre>
      ) : pdf ? (
        <PdfPreview data={pdf} />
      ) : (
        <div className="dock-empty">Typesetting…</div>
      )}
    </section>
  );
}

function HtmlDock() {
  const book = useBook((s) => s.book);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const chapters = book?.chapters ?? [];
  const idx = chapters.findIndex((c) => c.id === activeChapterId);
  const chapter = chapters[idx] ?? chapters[0];
  const html = useMemo(() => (chapter ? generateHTML(chapter.content, editorExtensions) : ""), [chapter]);

  if (!book || !chapter) return null;

  return (
    <section className="dock">
      <div className="dock-head">
        <span className="label">Preview</span>
        <div className="dock-tools">
          <span className="meta">
            {idx + 1} of {chapters.length}
          </span>
          <TrimSelect />
        </div>
      </div>
      <div className="page">
        <div className="p-opener">
          <div className="p-num">Chapter {idx + 1}</div>
          <div className="p-title">{chapter.title || "Untitled"}</div>
        </div>
        <div className="page-body" dangerouslySetInnerHTML={{ __html: html }} />
        <div className="folio">{idx * 8 + 7}</div>
      </div>
    </section>
  );
}
