import type { KeyboardEvent, PointerEvent } from "react";
import { applyPaneWidth, currentPaneWidth, resetPaneWidth, type Pane } from "../panes";

const STEP = 16;

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

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const towards = pane === "sidebar" ? 1 : -1;
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      applyPaneWidth(pane, currentPaneWidth(pane) + (e.key === "ArrowRight" ? STEP : -STEP) * towards);
    } else if (e.key === "Home" || e.key === "Enter") {
      e.preventDefault();
      resetPaneWidth(pane);
    }
  };

  return (
    <div
      className="pane-resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label={pane === "sidebar" ? "Resize chapters panel" : "Resize preview panel"}
      tabIndex={0}
      onPointerDown={onPointerDown}
      onKeyDown={onKeyDown}
      onDoubleClick={() => resetPaneWidth(pane)}
      title="Drag to resize · double-click to reset"
    />
  );
}
