import { save, open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { Book } from "./model/book";

const FILTERS = [{ name: "margin book", extensions: ["margin"] }];

export async function chooseSavePath(book: Book): Promise<string | null> {
  return save({ filters: FILTERS, defaultPath: `${book.metadata.title || "Untitled"}.margin` });
}

export async function writeBook(book: Book, path: string): Promise<void> {
  await invoke("write_file", { path, contents: JSON.stringify(book, null, 2) });
}

export async function openBook(): Promise<{ book: Book; path: string } | null> {
  const selected = await open({ filters: FILTERS, multiple: false, directory: false });
  if (typeof selected !== "string") return null;
  const contents = await invoke<string>("read_file", { path: selected });
  return { book: JSON.parse(contents) as Book, path: selected };
}

function toBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

export async function saveBytes(
  bytes: Uint8Array,
  defaultName: string,
  filters: { name: string; extensions: string[] }[]
): Promise<void> {
  const path = await save({ filters, defaultPath: defaultName });
  if (!path) return;
  await invoke("write_bytes", { path, data: toBase64(bytes) });
}
