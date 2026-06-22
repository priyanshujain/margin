import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useBook } from "../store/useBook";
import { listSystemFonts } from "../ipc";
import {
  type BookFonts,
  type FontRef,
  BUNDLED_FONTS,
  FONT_PAIRINGS,
  decodeRef,
  encodeRef,
  fontStack,
  pairingFor,
} from "../model/fonts";
import { Icon } from "./Icon";

const LANGUAGES = [
  { value: "en", label: "English" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "de", label: "German" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
];

let systemFontCache: string[] | null = null;

interface Draft {
  title: string;
  subtitle: string;
  author: string;
  isbn: string;
  language: string;
  bleed: boolean;
  fonts: BookFonts;
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      {children}
    </label>
  );
}

function FontSelect({ value, system, onChange }: { value: FontRef; system: string[]; onChange: (ref: FontRef) => void }) {
  const extra = value.kind === "system" && !system.includes(value.family) ? [value.family] : [];
  return (
    <select value={encodeRef(value)} style={{ fontFamily: fontStack(value) }} onChange={(e) => onChange(decodeRef(e.target.value))}>
      <optgroup label="Bundled">
        {BUNDLED_FONTS.map((f) => (
          <option key={f.id} value={`b:${f.id}`}>
            {f.label}
          </option>
        ))}
      </optgroup>
      {(system.length > 0 || extra.length > 0) && (
        <optgroup label="System">
          {[...extra, ...system].map((name) => (
            <option key={name} value={`s:${name}`}>
              {name}
            </option>
          ))}
        </optgroup>
      )}
    </select>
  );
}

export function Settings({ onClose, onSave }: { onClose: () => void; onSave: () => void }) {
  const book = useBook((s) => s.book);
  const setMetadata = useBook((s) => s.setMetadata);
  const setSettings = useBook((s) => s.setSettings);

  const [draft, setDraft] = useState<Draft>(() => ({
    title: book?.metadata.title ?? "",
    subtitle: book?.metadata.subtitle ?? "",
    author: book?.metadata.author ?? "",
    isbn: book?.metadata.isbn ?? "",
    language: book?.metadata.language ?? "en",
    bleed: book?.settings.bleed ?? true,
    fonts: book?.settings.fonts ?? FONT_PAIRINGS[0],
  }));
  const [system, setSystem] = useState<string[]>(() => systemFontCache ?? []);
  const [advanced, setAdvanced] = useState(() => pairingFor(draft.fonts) === null);

  useEffect(() => {
    if (systemFontCache) return;
    listSystemFonts().then((fonts) => {
      systemFontCache = fonts;
      setSystem(fonts);
    });
  }, []);

  if (!book) return null;

  const set = (patch: Partial<Draft>) => setDraft((d) => ({ ...d, ...patch }));
  const setFont = (slot: "body" | "heading", ref: FontRef) => set({ fonts: { ...draft.fonts, [slot]: ref } });

  const activePreset = pairingFor(draft.fonts);
  const usesSystem = draft.fonts.body.kind === "system" || draft.fonts.heading.kind === "system";

  const save = () => {
    const { bleed, fonts, ...metadata } = draft;
    setMetadata(metadata);
    setSettings({ bleed, fonts });
    onSave();
    onClose();
  };

  return (
    <div className="overlay" onClick={onClose}>
      <div className="panel" onClick={(e) => e.stopPropagation()}>
        <div className="panel-head">
          <h2>Book setup</h2>
          <button className="icon-btn" onClick={onClose} title="Close">
            <Icon d="M6 6l12 12M18 6L6 18" />
          </button>
        </div>
        <div className="panel-body">
          <Field label="Title">
            <input value={draft.title} placeholder="Untitled" onChange={(e) => set({ title: e.target.value })} />
          </Field>
          <Field label="Subtitle">
            <input value={draft.subtitle} onChange={(e) => set({ subtitle: e.target.value })} />
          </Field>
          <Field label="Author">
            <input value={draft.author} onChange={(e) => set({ author: e.target.value })} />
          </Field>
          <Field label="ISBN">
            <input value={draft.isbn} placeholder="optional" onChange={(e) => set({ isbn: e.target.value })} />
          </Field>
          <Field label="Language">
            <select value={draft.language} onChange={(e) => set({ language: e.target.value })}>
              {LANGUAGES.map((l) => (
                <option key={l.value} value={l.value}>
                  {l.label}
                </option>
              ))}
            </select>
          </Field>

          <div className="field">
            <span className="field-label">Typography</span>
            <div className="font-presets">
              {FONT_PAIRINGS.map((p) => (
                <button
                  key={p.id}
                  type="button"
                  className="font-preset"
                  data-on={activePreset === p.id}
                  onClick={() => set({ fonts: { body: p.body, heading: p.heading } })}
                >
                  <span className="font-preset-demo" style={{ fontFamily: fontStack(p.heading) }}>
                    Ag
                  </span>
                  <span className="font-preset-name">{p.label}</span>
                </button>
              ))}
            </div>
            <button type="button" className="font-advanced" onClick={() => setAdvanced((v) => !v)}>
              {advanced ? "Hide custom fonts" : "Customize fonts"}
            </button>
          </div>

          {advanced && (
            <>
              <Field label="Body font">
                <FontSelect value={draft.fonts.body} system={system} onChange={(ref) => setFont("body", ref)} />
              </Field>
              <Field label="Heading font">
                <FontSelect value={draft.fonts.heading} system={system} onChange={(ref) => setFont("heading", ref)} />
              </Field>
              {usesSystem && (
                <p className="font-note">System fonts preview here but may not embed in exported PDF or EPUB, so they can look different on other devices.</p>
              )}
            </>
          )}

          <div className="font-sample" style={{ fontFamily: fontStack(draft.fonts.body) }}>
            <span className="font-sample-title" style={{ fontFamily: fontStack(draft.fonts.heading) }}>
              Chapter One
            </span>
            She read the first line twice, and the whole quiet house seemed to lean in to listen.
          </div>

          <label className="check">
            <input type="checkbox" checked={draft.bleed} onChange={(e) => set({ bleed: e.target.checked })} />
            Add bleed for full-page images (print)
          </label>
        </div>
        <div className="panel-foot">
          <button className="btn-primary" onClick={save}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
