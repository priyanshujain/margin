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

  if (!book) return null;
  const { metadata, settings } = book;

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
            <input value={metadata.title} placeholder="Untitled" onChange={(e) => setMetadata({ title: e.target.value })} />
          </Field>
          <Field label="Subtitle">
            <input value={metadata.subtitle} onChange={(e) => setMetadata({ subtitle: e.target.value })} />
          </Field>
          <Field label="Author">
            <input value={metadata.author} onChange={(e) => setMetadata({ author: e.target.value })} />
          </Field>
          <Field label="ISBN">
            <input value={metadata.isbn} placeholder="optional" onChange={(e) => setMetadata({ isbn: e.target.value })} />
          </Field>
          <Field label="Language">
            <select value={metadata.language} onChange={(e) => setMetadata({ language: e.target.value })}>
              {LANGUAGES.map((l) => (
                <option key={l.value} value={l.value}>
                  {l.label}
                </option>
              ))}
            </select>
          </Field>
          <label className="check">
            <input type="checkbox" checked={settings.bleed} onChange={(e) => setSettings({ bleed: e.target.checked })} />
            Add bleed for full-page images (print)
          </label>
        </div>
        <div className="panel-foot">
          <button
            className="btn-primary"
            onClick={() => {
              onSave();
              onClose();
            }}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
