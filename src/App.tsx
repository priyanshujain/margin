import { useState } from "react";

const CHAPTERS = [
  { n: "1", title: "The Lighthouse" },
  { n: "2", title: "Salt and Iron" },
  { n: "3", title: "What the Tide Left" },
  { n: "4", title: "Homecoming" },
];

const BODY = [
  "The lamp had not been lit in thirty years, and yet the islanders still set their clocks by a light that no longer came. Mara climbed the iron stair each evening out of a habit older than memory, and each evening she found the same dark glass waiting, patient as the sea.",
  "Her grandfather had kept the flame. Her mother had let it die. Between those two facts lay the whole of the family's quarrel with the water, and Mara had inherited both the quarrel and the key that opened the lantern room.",
  "She did not light it that night either. She only stood at the gallery rail and counted the boats coming in, the way she had been taught to count them, as though arithmetic alone could keep them safe until morning.",
];

function Icon({ d }: { d: string }) {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d={d} />
    </svg>
  );
}

function App() {
  const [active, setActive] = useState("1");
  const [dock, setDock] = useState(true);
  const current = CHAPTERS.find((c) => c.n === active)!;

  return (
    <div className="app">
      <header className="titlebar" data-tauri-drag-region>
        <span className="doc-title">Untitled — a novel</span>
        <div className="actions">
          <button className="icon-btn" data-on={dock} onClick={() => setDock(!dock)} title="Toggle preview">
            <Icon d="M3 4.5h18v15H3zM14 4.5v15" />
          </button>
        </div>
      </header>

      <div className="body">
        <aside className="sidebar">
          <div className="brand">
            <span className="mark">margin</span>
            <span className="dot" />
          </div>
          <div className="nav-label">Chapters</div>
          <ul className="chapters">
            {CHAPTERS.map((c) => (
              <li key={c.n} className="chapter" data-active={c.n === active} onClick={() => setActive(c.n)}>
                <span className="num">{c.n}</span>
                <span className="title">{c.title}</span>
              </li>
            ))}
          </ul>
          <button className="add-chapter">
            <Icon d="M12 5v14M5 12h14" />
            New chapter
          </button>
        </aside>

        <main className="editor-pane">
          <article className="sheet prose">
            <header className="chapter-opener">
              <div className="chapter-num">Chapter {current.n}</div>
              <h1 className="chapter-title">{current.title}</h1>
            </header>
            {BODY.map((t, i) => (
              <p key={i} className={i === 0 ? "first" : undefined}>{t}</p>
            ))}
            <div className="scene-break">⁂</div>
            <p>Morning came the color of pewter. The tide had left its usual accounting along the shingle — a torn net, a single boot, the pale architecture of a gull — and Mara walked the length of it without hurry, naming each thing as her grandfather had named them to her.</p>
          </article>
        </main>

        {dock && (
          <section className="dock">
            <div className="dock-head">
              <span className="label">Preview</span>
              <span className="meta">6 × 9 in · 1 of 1</span>
            </div>
            <div className="page">
              <div className="p-opener">
                <div className="p-num">Chapter {current.n}</div>
                <div className="p-title">{current.title}</div>
              </div>
              {BODY.map((t, i) => (
                <p key={i}>{t}</p>
              ))}
              <div className="folio">7</div>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

export default App;
