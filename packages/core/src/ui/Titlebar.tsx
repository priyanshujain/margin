import type { MouseEvent, ReactNode } from "react";
import "./Titlebar.css";

export interface TitlebarProps {
  lead?: ReactNode;
  center?: ReactNode;
  trail?: ReactNode;
}

export function quietDoubleClick(e: MouseEvent<HTMLElement>) {
  if (
    e.detail >= 2 &&
    e.target instanceof HTMLElement &&
    e.target.hasAttribute("data-tauri-drag-region")
  ) {
    e.preventDefault();
  }
}

export function Titlebar({ lead, center, trail }: TitlebarProps) {
  return (
    <header className="titlebar" data-tauri-drag-region onMouseDown={quietDoubleClick}>
      <div className="titlebar-lead" data-tauri-drag-region>
        {lead}
      </div>
      {center ?? <div data-tauri-drag-region />}
      <div className="titlebar-trail" data-tauri-drag-region>
        {trail}
      </div>
    </header>
  );
}
