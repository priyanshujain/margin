import type { Book } from "../model/book";
import { bookToPdfInputs } from "./typst";
import { buildEpub } from "./epub";
import { compilePdf } from "../ipc";
import { saveBytes } from "../project";

const PDF = [{ name: "PDF", extensions: ["pdf"] }];
const EPUB = [{ name: "EPUB", extensions: ["epub"] }];

export async function exportPdf(book: Book): Promise<void> {
  const { source, images } = bookToPdfInputs(book);
  const bytes = await compilePdf(source, images);
  await saveBytes(bytes, `${book.metadata.title || "Untitled"}.pdf`, PDF);
}

export async function exportEpub(book: Book): Promise<void> {
  const bytes = await buildEpub(book);
  await saveBytes(bytes, `${book.metadata.title || "Untitled"}.epub`, EPUB);
}
