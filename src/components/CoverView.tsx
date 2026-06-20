import { useRef } from "react";
import { COVER_PALETTES } from "../model/book";
import { useBook } from "../store/useBook";
import { CoverArt } from "./CoverArt";
import { Icon } from "./Icon";

function readImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

export function CoverView() {
  const book = useBook((s) => s.book);
  const setCover = useBook((s) => s.setCover);
  const fileRef = useRef<HTMLInputElement>(null);

  if (!book) return null;
  const cover = book.cover;

  const upload = async (file: File) => {
    const image = await readImage(file);
    setCover({ kind: "image", image });
  };

  return (
    <div className="cover-pane">
      <div className="cover-stage">
        <CoverArt book={book} />
      </div>

      <div className="cover-controls">
        {cover.kind === "image" ? (
          <>
            <div className="cover-actions">
              <button className="cover-btn" onClick={() => fileRef.current?.click()}>
                <Icon d="M4 5h16v14H4zM4 16l4.5-4.5 3 3L16 10l4 4" /> Replace image
              </button>
              <button className="cover-btn" onClick={() => setCover({ kind: "default", image: "" })}>
                <Icon d="M6 6l12 12M18 6L6 18" /> Remove
              </button>
            </div>
            <p className="cover-hint">Your artwork fills the cover as-is.</p>
          </>
        ) : (
          <>
            <div className="cover-palettes">
              {COVER_PALETTES.map((p) => (
                <button
                  key={p.id}
                  className="cover-swatch"
                  data-on={cover.bg === p.bg && cover.ink === p.ink}
                  style={{ background: p.bg, color: p.ink }}
                  title={p.label}
                  onClick={() => setCover({ bg: p.bg, ink: p.ink })}
                >
                  Aa
                </button>
              ))}
              <label className="cover-color" title="Background">
                <input type="color" value={cover.bg} onChange={(e) => setCover({ bg: e.target.value })} />
              </label>
              <label className="cover-color" title="Text">
                <input type="color" value={cover.ink} onChange={(e) => setCover({ ink: e.target.value })} />
              </label>
            </div>
            <div className="cover-actions">
              <button className="cover-btn" onClick={() => fileRef.current?.click()}>
                <Icon d="M4 5h16v14H4zM4 16l4.5-4.5 3 3L16 10l4 4" /> Upload image
              </button>
            </div>
            <p className="cover-hint">Title, subtitle &amp; author are set in Book setup.</p>
          </>
        )}
        <input
          ref={fileRef}
          type="file"
          accept="image/*"
          hidden
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) upload(file);
            e.currentTarget.value = "";
          }}
        />
      </div>
    </div>
  );
}
