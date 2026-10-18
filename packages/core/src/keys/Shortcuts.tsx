import type { ReactNode } from "react";
import { Key } from "../ui/Key";
import { Sheet } from "../ui/Sheet";
import { allBindings, groupsOf, keyLabel, type Binding } from "./bindings";
import "./shortcuts.css";

export interface ShortcutsSheetProps {
  open: boolean;
  onClose: () => void;
  bindings?: readonly Binding[];
  groups?: readonly string[];
  note?: ReactNode;
  onBack?: () => void;
  backLabel?: string;
}

const NOTE =
  "Nothing is modal and nothing is chorded. Keys stand back while a text field has the focus, except the palette, which is how you get out of anywhere.";

export function ShortcutsSheet({
  open,
  onClose,
  bindings,
  groups,
  note = NOTE,
  onBack,
  backLabel,
}: ShortcutsSheetProps) {
  const table = bindings ?? allBindings();
  const order = groups ?? groupsOf(table);
  return (
    <Sheet
      open={open}
      title="Keyboard shortcuts"
      size="wide"
      onClose={onClose}
      onBack={onBack}
      backLabel={backLabel}
    >
      <div className="shortcuts">
        {order.map((group) => {
          const rows = table.filter((binding) => binding.group === group && binding.keys.length > 0);
          if (rows.length === 0) return null;
          return (
            <section className="shortcuts-group" key={group}>
              <h3 className="shortcuts-heading">{group}</h3>
              <dl className="shortcuts-list">
                {rows.map((binding) => (
                  <div className="shortcuts-row" key={`${binding.context}:${binding.keys.join(" ")}`}>
                    <dt className="shortcuts-keys">
                      {binding.keys.map((key) => (
                        <Key key={key}>{keyLabel(key)}</Key>
                      ))}
                    </dt>
                    <dd className="shortcuts-label">{binding.label}</dd>
                  </div>
                ))}
              </dl>
            </section>
          );
        })}
      </div>
      {note ? <p className="shortcuts-note">{note}</p> : null}
    </Sheet>
  );
}

