import { useEffect, useRef, useState } from "react";
import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export function PdfPreview({ data }: { data: Uint8Array }) {
  const ref = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const container = ref.current;
    if (!container) return;
    let cancelled = false;
    const loadingTask = pdfjs.getDocument({ data: data.slice() });

    (async () => {
      try {
        const doc = await loadingTask.promise;
        if (cancelled) return;
        const width = container.clientWidth;
        const dpr = window.devicePixelRatio || 1;
        const canvases: HTMLCanvasElement[] = [];

        for (let n = 1; n <= doc.numPages; n++) {
          const page = await doc.getPage(n);
          if (cancelled) return;
          const base = page.getViewport({ scale: 1 });
          const scale = width / base.width;
          const viewport = page.getViewport({ scale: scale * dpr });
          const canvas = document.createElement("canvas");
          canvas.className = "pdf-page";
          canvas.width = viewport.width;
          canvas.height = viewport.height;
          canvas.style.width = "100%";
          await page.render({ canvas, viewport }).promise;
          if (cancelled) return;
          canvases.push(canvas);
        }
        container.replaceChildren(...canvases);
        setError(null);
      } catch (e) {
        if (cancelled) return;
        container.replaceChildren();
        setError(String(e));
      }
    })();

    return () => {
      cancelled = true;
      loadingTask.destroy();
    };
  }, [data]);

  return (
    <>
      {error && <pre className="dock-error">{error}</pre>}
      <div className="pdf-pages" ref={ref} />
    </>
  );
}
