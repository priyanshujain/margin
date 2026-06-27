import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Icon } from "./Icon";

interface RowMenuProps {
  label: string;
  onDuplicate?: () => void;
  onDelete: () => void;
  onToggleTitle?: () => void;
  titleHidden?: boolean;
  onToggleMargin?: () => void;
  marginHidden?: boolean;
  onOpenChange?: (open: boolean) => void;
  className?: string;
}

export function RowMenu({ label, onDuplicate, onDelete, onToggleTitle, titleHidden, onToggleMargin, marginHidden, onOpenChange, className = "" }: RowMenuProps) {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    onOpenChange?.(open);
  }, [open]);
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

  const toggleTitle = (e: React.MouseEvent) => {
    e.stopPropagation();
    setOpen(false);
    onToggleTitle?.();
  };

  const toggleMargin = (e: React.MouseEvent) => {
    e.stopPropagation();
    setOpen(false);
    onToggleMargin?.();
  };

  const duplicate = (e: React.MouseEvent) => {
    e.stopPropagation();
    setOpen(false);
    onDuplicate?.();
  };

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
            {onToggleTitle && (
              <button className="row-menu-item" onClick={toggleTitle}>
                {titleHidden ? (
                  <Icon d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z" size={14} />
                ) : (
                  <Icon d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24M1 1l22 22" size={14} />
                )}
                {titleHidden ? "Show title" : "Hide title"}
              </button>
            )}
            {onToggleMargin && (
              <button className="row-menu-item" onClick={toggleMargin}>
                {marginHidden ? (
                  <Icon d="M4 5h16 M12 10v10 M8 16l4 4 4-4" size={14} />
                ) : (
                  <Icon d="M4 5h16 M12 20V10 M8 14l4-4 4 4" size={14} />
                )}
                {marginHidden ? "Show top margin" : "Hide top margin"}
              </button>
            )}
            {onDuplicate && (
              <button className="row-menu-item" onClick={duplicate}>
                <Icon d="M9 9h11v11h-11z M6 15V5h9" size={14} />
                Duplicate
              </button>
            )}
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
