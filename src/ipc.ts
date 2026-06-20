import { invoke } from "@tauri-apps/api/core";

export interface ImageInput {
  path: string;
  data: string;
}

export async function compilePdf(source: string, images: ImageInput[] = []): Promise<Uint8Array> {
  const buffer = await invoke<ArrayBuffer>("compile_pdf", { source, images });
  return new Uint8Array(buffer);
}
