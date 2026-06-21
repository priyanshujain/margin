export type Width = "narrow" | "medium" | "wide" | "full";

const KEY = "margin-width";

const MEASURE: Record<Width, string> = {
  narrow: "38em",
  medium: "46em",
  wide: "62em",
  full: "80em",
};

export const WIDTH_OPTIONS: { id: Width; label: string }[] = [
  { id: "narrow", label: "Narrow" },
  { id: "medium", label: "Medium" },
  { id: "wide", label: "Wide" },
  { id: "full", label: "Full" },
];

export function initialWidth(): Width {
  const saved = localStorage.getItem(KEY);
  if (saved && saved in MEASURE) return saved as Width;
  return "medium";
}

export function applyWidth(width: Width) {
  document.documentElement.style.setProperty("--measure", MEASURE[width]);
  localStorage.setItem(KEY, width);
}
