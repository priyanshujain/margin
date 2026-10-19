import { call } from "./ipc";
import type { Book } from "./model/book";

const FILTERS = [{ name: "margin project", extensions: ["margin"] }];

export async function chooseSavePath(book: Book): Promise<string | null> {
  return call<string | null>("pick_save_path", {
    defaultName: `${book.metadata.title || "Untitled"}.margin`,
    filters: FILTERS,
  });
}

export async function writeBook(book: Book, path: string): Promise<void> {
  await call("write_file", { path, contents: JSON.stringify(book, null, 2) });
}

export async function openBook(): Promise<{ book: Book; path: string } | null> {
  const selected = await call<string | null>("pick_open_path", { filters: FILTERS });
  if (!selected) return null;
  const contents = await call<string>("read_file", { path: selected });
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
  const path = await call<string | null>("pick_save_path", { defaultName, filters });
  if (!path) return;
  await call("write_bytes", { path, data: toBase64(bytes) });
}
