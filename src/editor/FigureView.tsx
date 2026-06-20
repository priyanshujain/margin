import { NodeViewWrapper, type NodeViewProps } from "@tiptap/react";

const PLACEMENTS = [
  { key: "inline", label: "Inline" },
  { key: "full-width", label: "Full width" },
  { key: "full-page", label: "Full page" },
  { key: "float-top", label: "Float" },
];

export function FigureView({ node, updateAttributes, selected }: NodeViewProps) {
  const { src, alt, caption, placement } = node.attrs;

  return (
    <NodeViewWrapper className="figure" data-placement={placement} data-selected={selected} contentEditable={false}>
      <div className="figure-frame">
        {src ? <img src={src} alt={alt} /> : <div className="figure-empty">No image</div>}
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
        </div>
      )}
    </NodeViewWrapper>
  );
}
