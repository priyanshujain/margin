import { useMemo } from "react";
import { generateHTML } from "@tiptap/core";
import { useBook } from "../store/useBook";
import { editorExtensions } from "../editor/extensions";

const TRIM_LABEL: Record<string, string> = {
  "6x9": "6 × 9 in",
  "5.5x8.5": "5.5 × 8.5 in",
  "5x8": "5 × 8 in",
  a5: "A5",
};

export function Dock() {
  const chapters = useBook((s) => s.book.chapters);
  const trim = useBook((s) => s.book.settings.trim);
  const activeChapterId = useBook((s) => s.activeChapterId);

  const idx = chapters.findIndex((c) => c.id === activeChapterId);
  const chapter = chapters[idx];
  const html = useMemo(() => generateHTML(chapter.content, editorExtensions), [chapter.content]);

  return (
    <section className="dock">
      <div className="dock-head">
        <span className="label">Preview</span>
        <span className="meta">
          {TRIM_LABEL[trim]} · {idx + 1} of {chapters.length}
        </span>
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
