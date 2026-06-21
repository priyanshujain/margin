import { useRef, useState } from "react";
import { NodeViewWrapper, type NodeViewProps } from "@tiptap/react";
import { FIGURE_WIDTH, isResizablePlacement, type FigurePlacement } from "../model/book";

const PLACEMENTS = [
  { key: "inline", label: "Inline" },
  { key: "full-width", label: "Full width" },
  { key: "full-page", label: "Full page" },
  { key: "float-top", label: "Float" },
];

const clampWidth = (value: number) => Math.min(100, Math.max(20, Math.round(value)));

export function FigureView({ node, updateAttributes, selected }: NodeViewProps) {
  const { src, alt, caption, placement } = node.attrs;
  const frameRef = useRef<HTMLDivElement>(null);
  const [dragWidth, setDragWidth] = useState<number | null>(null);

  const resizable = isResizablePlacement(placement as FigurePlacement);
  const baseWidth = node.attrs.width ?? FIGURE_WIDTH[placement as FigurePlacement] ?? 100;
  const width = resizable ? (dragWidth ?? baseWidth) : null;

  const startResize = (e: React.PointerEvent) => {
    e.preventDefault();
    const wrapper = frameRef.current?.closest(".figure") as HTMLElement | null;
    const container = wrapper?.parentElement;
    if (!wrapper || !container) return;
    const startX = e.clientX;
    const startPx = wrapper.getBoundingClientRect().width;
    const containerPx = container.clientWidth || 1;
    const percentAt = (clientX: number) => clampWidth(((startPx + clientX - startX) / containerPx) * 100);

    const onMove = (ev: PointerEvent) => setDragWidth(percentAt(ev.clientX));
    const onUp = (ev: PointerEvent) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      updateAttributes({ width: percentAt(ev.clientX) });
      setDragWidth(null);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  return (
    <NodeViewWrapper
      className="figure"
      data-placement={placement}
      data-selected={selected}
      style={width != null ? { width: `${width}%` } : undefined}
      contentEditable={false}
    >
      <div className="figure-frame" ref={frameRef}>
        {src ? <img src={src} alt={alt} /> : <div className="figure-empty">No image</div>}
        {selected && src && resizable && (
          <div className="figure-handle" onPointerDown={startResize} title="Drag to resize" />
        )}
      </div>
      <input
        className="figure-caption"
        value={caption}
        placeholder="Add a caption…"
        onChange={(e) => updateAttributes({ caption: e.target.value })}
      />
      {selected && (
        <div className="figure-controls">
          {PLACEMENTS.map((p) => (
            <button key={p.key} data-on={placement === p.key} onClick={() => updateAttributes({ placement: p.key })}>
              {p.label}
            </button>
          ))}
          {resizable && src && <span className="figure-width">{Math.round(width ?? baseWidth)}%</span>}
        </div>
      )}
    </NodeViewWrapper>
  );
}
