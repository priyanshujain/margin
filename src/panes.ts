export type Pane = "sidebar" | "dock";

const VAR: Record<Pane, string> = {
  sidebar: "--pane-sidebar",
  dock: "--pane-dock",
};

const KEY: Record<Pane, string> = {
  sidebar: "margin-pane-sidebar",
  dock: "margin-pane-dock",
};

const DEFAULT: Record<Pane, number> = {
  sidebar: 248,
  dock: 384,
};

const MIN: Record<Pane, number> = {
  sidebar: 200,
  dock: 280,
};

const MAX: Record<Pane, number> = {
  sidebar: 460,
  dock: 720,
};

function clamp(pane: Pane, px: number): number {
  return Math.round(Math.min(MAX[pane], Math.max(MIN[pane], px)));
}

export function currentPaneWidth(pane: Pane): number {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(VAR[pane]);
  const n = parseInt(raw, 10);
  return Number.isFinite(n) && n > 0 ? n : DEFAULT[pane];
}

export function applyPaneWidth(pane: Pane, px: number): void {
  const w = clamp(pane, px);
  document.documentElement.style.setProperty(VAR[pane], `${w}px`);
  localStorage.setItem(KEY[pane], String(w));
}

export function resetPaneWidth(pane: Pane): void {
  document.documentElement.style.setProperty(VAR[pane], `${DEFAULT[pane]}px`);
  localStorage.removeItem(KEY[pane]);
}
