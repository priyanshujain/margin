// margin-core's Sheet with Calendar's way back: Escape and the back button retrace the overlay
// trail, so Settings into Accounts returns to Settings rather than closing everything.

import type { ReactNode } from "react";
import { Sheet as CoreSheet } from "@margin/core/ui";
import { useOverlays } from "../store/useOverlays";
import "../styles/overlays.css";

export { Confirm } from "@margin/core/ui";

export interface SheetProps {
  open: boolean;
  title: string;
  onClose: () => void;
  /** `mini` is the month popover width, `wide` the editor. */
  size?: "mini" | "default" | "wide";
  foot?: ReactNode;
  children: ReactNode;
}

const TITLES: Record<string, string> = {
  palette: "the command palette",
  search: "search",
  "mini-month": "the calendar",
  editor: "the event",
  shortcuts: "shortcuts",
  menu: "the menu",
};

/** The way back to the panel this one was reached from, when there is one. */
export function useBack(): { onBack?: () => void; backLabel: string } {
  const trail = useOverlays((s) => s.trail);
  const back = useOverlays((s) => s.back);
  const previous = trail[trail.length - 1];
  return {
    onBack: previous ? back : undefined,
    backLabel: TITLES[previous ?? ""] ?? "the last panel",
  };
}

export function Sheet({ open, title, onClose, size = "default", foot, children }: SheetProps) {
  const { onBack, backLabel } = useBack();

  return (
    <CoreSheet
      open={open}
      title={title}
      onClose={onClose}
      size={size}
      foot={foot}
      onBack={onBack}
      backLabel={backLabel}
    >
      {children}
    </CoreSheet>
  );
}
