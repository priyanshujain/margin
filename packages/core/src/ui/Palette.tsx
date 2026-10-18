import { useEffect, useRef } from "react";
import { useEscapeLayer } from "../hooks/escape";
import { NO_AUTOFILL } from "./autofill";
import { Key } from "./Key";
import "./Palette.css";

export interface PaletteItem {
  id: string;
  label: string;
  hint?: string;
  keys?: string[];
}

export interface PaletteGroup {
  id: string;
  label: string;
  items: PaletteItem[];
}

export interface PaletteProps {
  open: boolean;
  query: string;
  onQuery: (query: string) => void;
  groups: PaletteGroup[];
  activeId?: string;
  onChoose: (id: string) => void;
  onClose: () => void;
  placeholder?: string;
  empty?: string;
}

export function Palette({
  open,
  query,
  onQuery,
  groups,
  activeId,
  onChoose,
  onClose,
  placeholder = "Run a command",
  empty = "Nothing matches",
}: PaletteProps) {
  useEscapeLayer(open, onClose);
  const input = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (open) input.current?.focus();
  }, [open]);

  if (!open) return null;

  const anything = groups.some((g) => g.items.length > 0);

  return (
    <div
      className="overlay"
      data-align="top"
      onPointerDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="panel palette" role="dialog" aria-modal="true" aria-label="Command palette">
        <input
          ref={input}
          className="palette-input"
          type="text"
          value={query}
          placeholder={placeholder}
          aria-label={placeholder}
          {...NO_AUTOFILL}
          onChange={(e) => onQuery(e.target.value)}
        />
        {anything ? (
          <ul className="palette-list" role="listbox">
            {groups
              .filter((g) => g.items.length > 0)
              .map((g) => (
                <li key={g.id} role="presentation">
                  <div className="palette-group">{g.label}</div>
                  <ul className="palette-items" role="group" aria-label={g.label}>
                    {g.items.map((it) => (
                      <li
                        key={it.id}
                        className="palette-row"
                        role="option"
                        aria-selected={it.id === activeId}
                        data-active={it.id === activeId ? "" : undefined}
                        onClick={() => onChoose(it.id)}
                      >
                        <span className="palette-label">
                          {it.label}
                          {it.hint ? <span className="hint">{it.hint}</span> : null}
                        </span>
                        {it.keys?.length ? (
                          <span className="palette-keys">
                            {it.keys.map((k) => (
                              <Key key={k} size="md">
                                {k}
                              </Key>
                            ))}
                          </span>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                </li>
              ))}
          </ul>
        ) : (
          <p className="palette-empty">{empty}</p>
        )}
      </div>
    </div>
  );
}
