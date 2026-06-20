import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Icon } from "./Icon";

interface RowMenuProps {
  label: string;
  onDelete: () => void;
  className?: string;
}

export function RowMenu({ label, onDelete, className = "" }: RowMenuProps) {
  const [open, setOpen] = useState(false);
  const [coords, setCoords] = useState({ top: 0, right: 0 });
  const btnRef = useRef<HTMLButtonElement>(null);
  const popRef = useRef<HTMLDivElement>(null);

  const toggle = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!open && btnRef.current) {
      const r = btnRef.current.getBoundingClientRect();
      setCoords({ top: r.bottom + 4, right: window.innerWidth - r.right });
    }
    setOpen((v) => !v);
  };

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (btnRef.current?.contains(e.target as Node) || popRef.current?.contains(e.target as Node)) return;
      setOpen(false);
    };
    const close = () => setOpen(false);
    document.addEventListener("mousedown", onDown, true);
    document.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      document.removeEventListener("mousedown", onDown, true);
      document.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  }, [open]);

  const remove = (e: React.MouseEvent) => {
    e.stopPropagation();
    setOpen(false);
    onDelete();
  };

  return (
    <>
      <button
        ref={btnRef}
        className={`row-menu-btn ${className}`}
        data-open={open}
        title={label}
        onMouseDown={(e) => e.stopPropagation()}
        onClick={toggle}
      >
        <Icon d="M12 5h.01M12 12h.01M12 19h.01" />
      </button>
      {open &&
        createPortal(
          <div ref={popRef} className="row-menu-pop" style={{ top: coords.top, right: coords.right }}>
            <button className="row-menu-item danger" onClick={remove}>
              <Icon d="M5 7h14M10 7V5h4v2M7 7l1 13h8l1-13M10 11v6M14 11v6" size={14} />
              Delete
            </button>
          </div>,
          document.body
        )}
    </>
  );
}
