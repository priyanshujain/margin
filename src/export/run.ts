import type { Book } from "../model/book";
import { exportEpub, exportPdf } from "./exporters";
import { unsupportedScripts } from "./typst";
import { useBook } from "../store/useBook";
import { isDesktop } from "../ipc";

const DESKTOP_ONLY =
  'Export runs in the desktop app only — open the window from "pnpm tauri dev" (you are viewing the browser preview).';

const FORMATS: Record<string, { label: string; run: (book: Book) => Promise<void> }> = {
  pdf: { label: "PDF", run: exportPdf },
  epub: { label: "EPUB", run: exportEpub },
};

function nextPaint(): Promise<void> {
  return new Promise((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
  );
}

export async function runExport(format: "pdf" | "epub"): Promise<void> {
  const { book, setExporting, setNotice } = useBook.getState();
  const { label, run } = FORMATS[format];
  if (!isDesktop) {
    setNotice(DESKTOP_ONLY);
    return;
  }
  if (!book) return;
  setExporting(label);
  await nextPaint();
  try {
    await run(book);
    if (format === "pdf") {
      const scripts = unsupportedScripts(book);
      if (scripts.length)
        setNotice(`Some ${scripts.join(", ")} characters may not render — the PDF fonts cover Latin scripts only.`);
    }
  } catch (e) {
    setNotice(`${label} export failed: ${e}`);
  } finally {
    setExporting(null);
  }
}
