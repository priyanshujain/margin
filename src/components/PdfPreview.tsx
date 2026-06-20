import { useEffect, useRef } from "react";
import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export function PdfPreview({ data }: { data: Uint8Array }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = ref.current;
    if (!container) return;
    let cancelled = false;

    (async () => {
      const doc = await pdfjs.getDocument({ data: data.slice() }).promise;
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
        await page.render({ canvasContext: canvas.getContext("2d")!, viewport }).promise;
        if (cancelled) return;
        canvases.push(canvas);
      }
      container.replaceChildren(...canvases);
    })();

    return () => {
      cancelled = true;
    };
  }, [data]);

  return <div className="pdf-pages" ref={ref} />;
}
