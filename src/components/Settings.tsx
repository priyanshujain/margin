import { useState } from "react";
import type { ReactNode } from "react";
import { useBook } from "../store/useBook";
import { Icon } from "./Icon";

const LANGUAGES = [
  { value: "en", label: "English" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "de", label: "German" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
];

interface Draft {
  title: string;
  subtitle: string;
  author: string;
  isbn: string;
  language: string;
  bleed: boolean;
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      {children}
    </label>
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
  }));

  if (!book) return null;

  const set = (patch: Partial<Draft>) => setDraft((d) => ({ ...d, ...patch }));

  const save = () => {
    const { bleed, ...metadata } = draft;
    setMetadata(metadata);
    setSettings({ bleed });
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
