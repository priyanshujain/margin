import { createCall } from "@margin/core/ipc";

export { isDesktop, isMacDesktop, isTauri, live } from "@margin/core/ipc";

export const call = createCall({ mock: () => import("./dev/mockIpc").then((m) => m.mockCall) });

export type WritingTool = "Proofread" | "Rewrite";

export async function runWritingTool(tool: WritingTool): Promise<void> {
  await call("run_writing_tool", { tool });
}

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
  const buffer = await call<ArrayBuffer>("compile_pdf", {
    source,
    images,
    emitWarnings,
    bundledFonts: fonts.bundled,
    systemFonts: fonts.system,
  });
  return new Uint8Array(buffer);
}

export async function listSystemFonts(): Promise<string[]> {
  try {
    return await call<string[]>("list_system_fonts");
  } catch {
    return [];
  }
}
