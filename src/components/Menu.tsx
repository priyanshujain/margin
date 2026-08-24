import { useRef, type ReactNode } from "react";
import { useFocusTrap } from "../focus";

interface MenuProps {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
}

export function Menu({ open, onClose, children }: MenuProps) {
  const ref = useRef<HTMLDivElement>(null);

  useFocusTrap(ref, open);

  if (!open) return null;

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp" && e.key !== "Home" && e.key !== "End") return;
    e.preventDefault();
    const items = Array.from(ref.current?.querySelectorAll<HTMLElement>("button") ?? []);
    if (!items.length) return;
    const at = items.indexOf(document.activeElement as HTMLElement);
    const next =
      e.key === "Home"
        ? 0
        : e.key === "End"
          ? items.length - 1
          : e.key === "ArrowDown"
            ? (at + 1) % items.length
            : (at - 1 + items.length) % items.length;
    items[next]?.focus();
  };

  return (
    <>
      <div className="menu-backdrop" onClick={onClose} />
      <div ref={ref} className="menu" onKeyDown={onKeyDown}>
        {children}
      </div>
    </>
  );
}
