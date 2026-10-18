import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { useEscapeLayer } from "../hooks/escape";
import "./Popover.css";

export type PopoverPlacement = "bottom-start" | "bottom-end" | "top-end";

export interface PopoverProps {
  open: boolean;
  anchor: HTMLElement | null;
  onClose: () => void;
  placement?: PopoverPlacement;
  width?: number;
  label?: string;
  children: ReactNode;
}

const EDGE = 8;

export function Popover({
  open,
  anchor,
  onClose,
  placement = "bottom-start",
  width = 320,
  label,
  children,
}: PopoverProps) {
  const panel = useRef<HTMLDivElement | null>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);

  useEscapeLayer(open, onClose);

  useLayoutEffect(() => {
    if (!open || !anchor) {
      setPos(null);
      return;
    }
    const place = () => {
      const r = anchor.getBoundingClientRect();
      const wanted = placement === "bottom-start" ? r.left : r.right - width;
      const left = Math.min(Math.max(EDGE, wanted), window.innerWidth - width - EDGE);
      const top = placement === "top-end" ? r.top - 6 : r.bottom + 6;
      setPos((was) => (was && was.left === left && was.top === top ? was : { left, top }));
    };
    place();
    window.addEventListener("scroll", place, true);
    window.addEventListener("resize", place);
    const ro = new ResizeObserver(place);
    ro.observe(document.body);
    return () => {
      window.removeEventListener("scroll", place, true);
      window.removeEventListener("resize", place);
      ro.disconnect();
    };
  }, [open, anchor, placement, width]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => {
      const t = e.target as Node;
      if (panel.current?.contains(t) || anchor?.contains(t)) return;
      onClose();
    };
    window.addEventListener("pointerdown", onDown, true);
    return () => window.removeEventListener("pointerdown", onDown, true);
  }, [open, anchor, onClose]);

  if (!open || !pos) return null;

  const place = {
    "--pop-left": `${pos.left}px`,
    "--pop-top": `${pos.top}px`,
    "--pop-w": `${width}px`,
  } as CSSProperties;

  return (
    <div
      className="popover"
      ref={panel}
      role="dialog"
      aria-label={label}
      data-placement={placement}
      style={place}
    >
      {children}
    </div>
  );
}
