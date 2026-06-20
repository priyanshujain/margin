import { useEffect, useRef, useState } from "react";
import { PAGE_TYPES } from "../model/book";
import { Icon } from "./Icon";

interface AddPageMenuProps {
  onAdd: (group: "front" | "back", title: string) => void;
}

const GROUPS: { key: "front" | "back"; label: string }[] = [
  { key: "front", label: "Front matter" },
  { key: "back", label: "Back matter" },
];

export function AddPageMenu({ onAdd }: AddPageMenuProps) {
  const [open, setOpen] = useState(false);
  const [coords, setCoords] = useState({ top: 0, left: 0 });
  const btnRef = useRef<HTMLButtonElement>(null);

  const toggle = () => {
    if (!open && btnRef.current) {
      const r = btnRef.current.getBoundingClientRect();
      setCoords({ top: r.bottom + 4, left: r.left });
    }
    setOpen((v) => !v);
  };

  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    document.addEventListener("mousedown", close);
    document.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  }, [open]);

  const pick = (group: "front" | "back", label: string) => {
    setOpen(false);
    onAdd(group, label);
  };

  return (
    <>
      <button ref={btnRef} className="add-page" data-open={open} onClick={toggle}>
        <Icon d="M12 5v14M5 12h14" size={15} />
        Add page
      </button>
      {open && (
        <div
          className="add-page-pop"
          style={{ top: coords.top, left: coords.left }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          {GROUPS.map((g) => (
            <div key={g.key} className="add-page-group">
              <div className="add-page-label">{g.label}</div>
              {PAGE_TYPES.filter((t) => t.group === g.key).map((t) => (
                <button key={t.id} className="add-page-item" onClick={() => pick(t.group, t.label)}>
                  {t.label}
                </button>
              ))}
            </div>
          ))}
        </div>
      )}
    </>
  );
}
