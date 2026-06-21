import type { PointerEvent } from "react";
import { applyPaneWidth, currentPaneWidth, resetPaneWidth, type Pane } from "../panes";

export function ResizeHandle({ pane }: { pane: Pane }) {
  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    const handle = e.currentTarget;
    const startX = e.clientX;
    const startWidth = currentPaneWidth(pane);
    handle.setPointerCapture(e.pointerId);
    document.body.classList.add("resizing");

    const onMove = (ev: globalThis.PointerEvent) => {
      const delta = ev.clientX - startX;
      applyPaneWidth(pane, pane === "sidebar" ? startWidth + delta : startWidth - delta);
    };
    const onUp = () => {
      document.body.classList.remove("resizing");
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
    };
    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
  };

  return (
    <div
      className="pane-resizer"
      role="separator"
      aria-orientation="vertical"
      onPointerDown={onPointerDown}
      onDoubleClick={() => resetPaneWidth(pane)}
      title="Drag to resize · double-click to reset"
    />
  );
}
