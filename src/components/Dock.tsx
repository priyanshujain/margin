import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { generateHTML } from "@tiptap/core";
import { COVER_ID, useBook } from "../store/useBook";
import { type Chapter, type TrimSize, bodyNumber, chapterKind } from "../model/book";
import { editorExtensions } from "../editor/extensions";
import { chapterToPdfInputs, coverToPdfInputs } from "../export/typst";
import { compilePdf, isDesktop } from "../ipc";
import { DEVICES, type Device, findDevice } from "../devices";
import { usePreviewMode } from "../store/usePreviewMode";
import { PdfPreview } from "./PdfPreview";
import { CoverArt } from "./CoverArt";
import { DeviceFrame } from "./DeviceFrame";

const TRIMS: { value: TrimSize; label: string }[] = [
  { value: "6x9", label: "6 × 9 in" },
  { value: "5.5x8.5", label: "5.5 × 8.5 in" },
  { value: "5x8", label: "5 × 8 in" },
  { value: "a5", label: "A5" },
];

function PreviewSelect() {
  const trim = useBook((s) => s.book?.settings.trim ?? "6x9");
  const setSettings = useBook((s) => s.setSettings);
  const mode = usePreviewMode((s) => s.mode);
  const setMode = usePreviewMode((s) => s.setMode);
  const value = mode === "print" ? trim : mode;
  return (
    <select
      className="trim-select"
      value={value}
      onChange={(e) => {
        const v = e.target.value;
        if (findDevice(v)) {
          setMode(v as Device["id"]);
        } else {
          setSettings({ trim: v as TrimSize });
          setMode("print");
        }
      }}
      title="Preview format"
    >
      <optgroup label="Print">
        {TRIMS.map((t) => (
          <option key={t.value} value={t.value}>
            {t.label}
          </option>
        ))}
      </optgroup>
      <optgroup label="Devices">
        {DEVICES.map((d) => (
          <option key={d.id} value={d.id}>
            {d.label}
          </option>
        ))}
      </optgroup>
    </select>
  );
}

function DockHead({ meta }: { meta?: ReactNode }) {
  return (
    <div className="dock-head">
      <span className="label">Preview</span>
      <div className="dock-tools">
        {meta}
        <PreviewSelect />
      </div>
    </div>
  );
}

function chapterEyebrow(chapter: Chapter, chapters: Chapter[], idx: number): string {
  const kind = chapterKind(chapter);
  return kind === "body" ? `Chapter ${bodyNumber(chapters, idx) ?? ""}` : kind === "front" ? "Front matter" : "Back matter";
}

function useActiveChapter() {
  const book = useBook((s) => s.book);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const chapters = book?.chapters ?? [];
  const idx = chapters.findIndex((c) => c.id === activeChapterId);
  const chapter = chapters[idx] ?? chapters[0];
  const html = useMemo(() => (chapter ? generateHTML(chapter.content, editorExtensions) : ""), [chapter]);
  const eyebrow = chapter ? chapterEyebrow(chapter, chapters, idx) : "";
  return { book, coverActive: activeChapterId === COVER_ID, chapters, idx, chapter, html, eyebrow };
}

export function Dock() {
  const mode = usePreviewMode((s) => s.mode);
  const device = findDevice(mode);
  if (device) return <DeviceDock device={device} />;
  return isDesktop ? <PdfDock /> : <HtmlDock />;
}

function PdfDock() {
  const book = useBook((s) => s.book);
  const activeChapterId = useBook((s) => s.activeChapterId);
  const [pdf, setPdf] = useState<Uint8Array | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    if (!book) return;
    const coverActive = activeChapterId === COVER_ID;
    const idx = Math.max(0, book.chapters.findIndex((c) => c.id === activeChapterId));
    clearTimeout(timer.current);
    setBusy(true);
    timer.current = setTimeout(() => {
      const { source, images } = coverActive ? coverToPdfInputs(book) : chapterToPdfInputs(book, idx);
      compilePdf(source, images)
        .then((bytes) => {
          setPdf(bytes);
          setError(null);
        })
        .catch((e) => setError(String(e)))
        .finally(() => setBusy(false));
    }, 350);
    return () => clearTimeout(timer.current);
  }, [book, activeChapterId]);

  return (
    <section className="dock">
      <DockHead meta={busy ? <span className="meta">updating…</span> : null} />
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
  const { book, coverActive, chapters, idx, chapter, html, eyebrow } = useActiveChapter();

  if (!book) return null;

  if (coverActive) {
    return (
      <section className="dock">
        <DockHead meta={<span className="meta">Cover</span>} />
        <div className="cover-stage">
          <CoverArt book={book} />
        </div>
      </section>
    );
  }

  if (!chapter) return null;

  return (
    <section className="dock">
      <DockHead
        meta={
          <span className="meta">
            {idx + 1} of {chapters.length}
          </span>
        }
      />
      <div className="page">
        <div className="p-opener">
          <div className="p-num">{eyebrow}</div>
          {!chapter.noTitle && <div className="p-title">{chapter.title || "Untitled"}</div>}
        </div>
        <div className="page-body" dangerouslySetInnerHTML={{ __html: html }} />
        <div className="folio">{idx * 8 + 7}</div>
      </div>
    </section>
  );
}

function DeviceDock({ device }: { device: Device }) {
  const { book, coverActive, chapters, idx, chapter, html, eyebrow } = useActiveChapter();

  if (!book) return null;

  if (coverActive) {
    return (
      <section className="dock">
        <DockHead meta={<span className="meta">Cover</span>} />
        <DeviceFrame device={device} cover={book} />
      </section>
    );
  }

  if (!chapter) {
    return (
      <section className="dock">
        <DockHead />
        <div className="dock-empty">No chapter selected.</div>
      </section>
    );
  }

  return (
    <section className="dock">
      <DockHead
        meta={
          <span className="meta">
            {idx + 1} of {chapters.length}
          </span>
        }
      />
      <DeviceFrame
        device={device}
        eyebrow={eyebrow}
        title={chapter.noTitle ? undefined : chapter.title || "Untitled"}
        html={html}
      />
    </section>
  );
}
