import { useEffect, useLayoutEffect, useRef, useState } from "react";
import * as pdfjs from "pdfjs-dist";
import type { PDFDocumentProxy, RenderTask } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { useBook } from "../store/useBook";
import { useExportPreview } from "../store/useExportPreview";
import { bookToPdfInputs, unsupportedScripts } from "../export/typst";
import { compilePdf } from "../ipc";
import { saveBytes } from "../project";
import { Icon } from "./Icon";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

const PDF_FILTER = [{ name: "PDF", extensions: ["pdf"] }];
const ZOOM_MIN = 0.5;
const ZOOM_MAX = 2.5;
const ZOOM_STEP = 0.25;

interface Frame {
  left: number;
  top: number;
  width: number;
  height: number;
}

function measureEditorPane(): Frame | null {
  const pane = document.querySelector(".editor-pane");
  if (!pane) return null;
  const r = pane.getBoundingClientRect();
  return {
    left: Math.round(r.left),
    top: Math.round(r.top),
    width: Math.round(r.width),
    height: Math.round(r.height),
  };
}

export function ExportPreview() {
  const open = useExportPreview((s) => s.open);
  const book = useBook((s) => s.book);
  if (!open || !book) return null;
  return <ExportPreviewModal />;
}

function ExportPreviewModal() {
  const book = useBook((s) => s.book)!;
  const close = useExportPreview((s) => s.close);
  const [pdf, setPdf] = useState<Uint8Array | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pages, setPages] = useState(0);
  const [zoom, setZoom] = useState(1);
  const [saving, setSaving] = useState(false);
  const [frame, setFrame] = useState<Frame | null>(measureEditorPane);
  const panelRef = useRef<HTMLDivElement>(null);
  const zoomRef = useRef(zoom);
  zoomRef.current = zoom;

  useLayoutEffect(() => {
    const update = () => setFrame(measureEditorPane());
    update();
    const pane = document.querySelector(".editor-pane");
    const observer = new ResizeObserver(update);
    if (pane) observer.observe(pane);
    window.addEventListener("resize", update);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", update);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    setPdf(null);
    setError(null);
    const { source, images, fonts } = bookToPdfInputs(book);
    compilePdf(source, images, false, fonts)
      .then((bytes) => !cancelled && setPdf(bytes))
      .catch((e) => !cancelled && setError(String(e)));
    return () => {
      cancelled = true;
    };
  }, [book]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close]);

  useEffect(() => {
    const el = panelRef.current;
    if (!el) return;
    const clamp = (z: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z));
    const snap = (z: number) => clamp(Math.round(z / ZOOM_STEP) * ZOOM_STEP);
    let start = 1;
    let accum = 0;

    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return;
      e.preventDefault();
      accum += e.deltaY;
      if (accum <= -40) {
        accum = 0;
        setZoom((z) => clamp(z + ZOOM_STEP));
      } else if (accum >= 40) {
        accum = 0;
        setZoom((z) => clamp(z - ZOOM_STEP));
      }
    };
    const onGestureStart = (e: Event) => {
      e.preventDefault();
      start = zoomRef.current;
    };
    const onGestureChange = (e: Event) => {
      e.preventDefault();
      const next = snap(start * (e as unknown as { scale: number }).scale);
      setZoom((z) => (z === next ? z : next));
    };
    const onGestureEnd = (e: Event) => e.preventDefault();

    el.addEventListener("wheel", onWheel, { passive: false });
    el.addEventListener("gesturestart", onGestureStart);
    el.addEventListener("gesturechange", onGestureChange);
    el.addEventListener("gestureend", onGestureEnd);
    return () => {
      el.removeEventListener("wheel", onWheel);
      el.removeEventListener("gesturestart", onGestureStart);
      el.removeEventListener("gesturechange", onGestureChange);
      el.removeEventListener("gestureend", onGestureEnd);
    };
  }, []);

  const scripts = unsupportedScripts(book);

  const save = async () => {
    if (!pdf) return;
    setSaving(true);
    try {
      await saveBytes(pdf, `${book.metadata.title || "Untitled"}.pdf`, PDF_FILTER);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const adjustZoom = (delta: number) =>
    setZoom((z) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.round((z + delta) * 100) / 100)));

  const compact = (frame?.width ?? 1000) < 460;

  return (
    <div className="overlay preview-overlay" onClick={close}>
      <div
        ref={panelRef}
        className="panel preview-panel"
        style={frame ? { position: "fixed", left: frame.left, top: frame.top, width: frame.width, height: frame.height } : undefined}
        onClick={(e) => e.stopPropagation()}
      >
        <header className="preview-bar">
          <button className="icon-btn" onClick={close} title="Close (Esc)">
            <Icon d="M6 6l12 12M18 6L6 18" />
          </button>
          <div className="preview-title">{book.metadata.title || "Untitled"}</div>
          {pdf && !compact && (
            <span className="preview-count">
              {pages} {pages === 1 ? "page" : "pages"}
            </span>
          )}
          <div className="preview-zoom">
            <button className="icon-btn" disabled={!pdf || zoom <= ZOOM_MIN} onClick={() => adjustZoom(-ZOOM_STEP)} title="Zoom out">
              <Icon d="M5 12h14" />
            </button>
            {!compact && <span>{Math.round(zoom * 100)}%</span>}
            <button className="icon-btn" disabled={!pdf || zoom >= ZOOM_MAX} onClick={() => adjustZoom(ZOOM_STEP)} title="Zoom in">
              <Icon d="M12 5v14M5 12h14" />
            </button>
          </div>
          <button className="btn-primary" disabled={!pdf || saving} onClick={save}>
            {saving ? "Saving…" : compact ? "Save" : "Save PDF…"}
          </button>
        </header>

        {pdf && scripts.length > 0 && (
          <div className="preview-warn">
            Some {scripts.join(", ")} characters may not render — the PDF fonts cover Latin scripts only.
          </div>
        )}

        {error ? (
          <div className="preview-stage">
            <pre className="dock-error">{error}</pre>
          </div>
        ) : pdf ? (
          <BookPages data={pdf} zoom={zoom} onPages={setPages} />
        ) : (
          <div className="preview-stage">
            <div className="preview-loading">
              <div className="spinner spinner-dark" />
              <p>Typesetting your book…</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function BookPages({ data, zoom, onPages }: { data: Uint8Array; zoom: number; onPages: (n: number) => void }) {
  const stageRef = useRef<HTMLDivElement>(null);
  const [doc, setDoc] = useState<PDFDocumentProxy | null>(null);
  const [baseRatio, setBaseRatio] = useState<number | null>(null);
  const [stageWidth, setStageWidth] = useState(0);

  useEffect(() => {
    let cancelled = false;
    const task = pdfjs.getDocument({ data: data.slice() });
    task.promise
      .then(async (d) => {
        if (cancelled) return;
        const first = (await d.getPage(1)).getViewport({ scale: 1 });
        if (cancelled) return;
        setDoc(d);
        setBaseRatio(first.height / first.width);
        onPages(d.numPages);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      task.destroy();
    };
  }, [data, onPages]);

  useEffect(() => {
    const stage = stageRef.current;
    if (!stage) return;
    const update = () => setStageWidth(stage.clientWidth);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(stage);
    return () => observer.disconnect();
  }, [doc]);

  const fit = Math.max(280, stageWidth - 56);
  const displayWidth = Math.round(fit * zoom);

  return (
    <div className="preview-stage" ref={stageRef}>
      {doc && baseRatio !== null && stageWidth > 0 && (
        <div className="preview-col">
          {Array.from({ length: doc.numPages }, (_, i) => (
            <PdfPage
              key={i + 1}
              doc={doc}
              pageNumber={i + 1}
              root={stageRef.current}
              displayWidth={displayWidth}
              baseRatio={baseRatio}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function PdfPage({
  doc,
  pageNumber,
  root,
  displayWidth,
  baseRatio,
}: {
  doc: PDFDocumentProxy;
  pageNumber: number;
  root: HTMLElement | null;
  displayWidth: number;
  baseRatio: number;
}) {
  const holder = useRef<HTMLDivElement>(null);
  const [show, setShow] = useState(false);
  const [ratio, setRatio] = useState(baseRatio);

  useEffect(() => {
    const el = holder.current;
    if (!el) return;
    const observer = new IntersectionObserver(([entry]) => setShow(entry.isIntersecting), {
      root,
      rootMargin: "1400px 0px",
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [root]);

  useEffect(() => {
    const el = holder.current;
    if (!el) return;
    if (!show) {
      el.replaceChildren();
      return;
    }
    let cancelled = false;
    let task: RenderTask | undefined;
    (async () => {
      try {
        const page = await doc.getPage(pageNumber);
        if (cancelled) return;
        const natural = page.getViewport({ scale: 1 });
        setRatio(natural.height / natural.width);
        const dpr = Math.min(window.devicePixelRatio || 1, 2);
        const viewport = page.getViewport({ scale: (displayWidth / natural.width) * dpr });
        const canvas = document.createElement("canvas");
        canvas.className = "preview-canvas";
        canvas.width = Math.ceil(viewport.width);
        canvas.height = Math.ceil(viewport.height);
        task = page.render({ canvas, viewport });
        await task.promise;
        if (cancelled) return;
        el.replaceChildren(canvas);
      } catch {
        // render cancelled or page failed; keep the previous canvas
      }
    })();
    return () => {
      cancelled = true;
      task?.cancel();
    };
  }, [show, doc, pageNumber, displayWidth]);

  return (
    <div
      ref={holder}
      className="preview-page"
      style={{ width: displayWidth, height: Math.round(displayWidth * ratio) }}
    />
  );
}
