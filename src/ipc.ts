import { invoke } from "@tauri-apps/api/core";

export const isDesktop = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface ImageInput {
  path: string;
  data: string;
}

export interface PdfFonts {
  bundled: string[];
  system: string[];
}

export async function compilePdf(
  source: string,
  images: ImageInput[] = [],
  emitWarnings = false,
  fonts: PdfFonts = { bundled: [], system: [] },
): Promise<Uint8Array> {
  const buffer = await invoke<ArrayBuffer>("compile_pdf", {
    source,
    images,
    emitWarnings,
    bundledFonts: fonts.bundled,
    systemFonts: fonts.system,
  });
  return new Uint8Array(buffer);
}

export async function listSystemFonts(): Promise<string[]> {
  if (!isDesktop) return [];
  try {
    return await invoke<string[]>("list_system_fonts");
  } catch {
    return [];
  }
}
