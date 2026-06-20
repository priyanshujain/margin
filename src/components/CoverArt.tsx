import { type Book, trimRatio } from "../model/book";

export function CoverArt({ book }: { book: Book }) {
  const { cover, metadata, settings } = book;
  const ratio = String(trimRatio(settings.trim));

  return (
    <div className="cover-art" style={{ aspectRatio: ratio }}>
      {cover.kind === "image" && cover.image ? (
        <img className="cover-face cover-face-image" src={cover.image} alt={metadata.title || "Cover"} />
      ) : (
        <div className="cover-face cover-face-default" style={{ background: cover.bg, color: cover.ink }}>
          <div className="cover-stack">
            <h1 className="cover-title">{metadata.title || "Untitled"}</h1>
            <span className="cover-rule" style={{ background: cover.ink }} />
            {metadata.subtitle && <p className="cover-subtitle">{metadata.subtitle}</p>}
          </div>
          {metadata.author && <p className="cover-author">{metadata.author}</p>}
        </div>
      )}
    </div>
  );
}
