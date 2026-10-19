import { type BookFonts, fontStack } from "./model/fonts";

export function applyBookFonts(fonts: BookFonts): void {
  const root = document.documentElement;
  root.style.setProperty("--font-book", fontStack(fonts.body));
  root.style.setProperty("--font-heading", fontStack(fonts.heading));
}

export function resetBookFonts(): void {
  const root = document.documentElement;
  root.style.removeProperty("--font-book");
  root.style.removeProperty("--font-heading");
}
