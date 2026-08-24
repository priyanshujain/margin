import { useEffect, useRef, type RefObject } from "react";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

function focusable(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (el) => !el.hidden && el.offsetParent !== null,
  );
}

let openTraps = 0;

export function focusTrapped(): boolean {
  return openTraps > 0;
}

export function useFocusTrap(ref: RefObject<HTMLElement | null>, active = true): void {
  const opener = useRef<HTMLElement | null>(null);
  const wasActive = useRef(false);
  if (active && !wasActive.current) opener.current = document.activeElement as HTMLElement | null;
  wasActive.current = active;

  useEffect(() => {
    const root = ref.current;
    if (!active || !root) return;
    openTraps++;
    if (!root.contains(document.activeElement)) focusable(root)[0]?.focus();

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const items = focusable(root);
      if (!items.length) return;
      const first = items[0];
      const last = items[items.length - 1];
      const current = document.activeElement as HTMLElement | null;
      if (!current || !root.contains(current)) {
        e.preventDefault();
        (e.shiftKey ? last : first).focus();
      } else if (e.shiftKey && current === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && current === last) {
        e.preventDefault();
        first.focus();
      }
    };

    root.addEventListener("keydown", onKeyDown);
    return () => {
      openTraps--;
      root.removeEventListener("keydown", onKeyDown);
      if (opener.current?.isConnected) opener.current.focus();
    };
  }, [ref, active]);
}
