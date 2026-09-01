import { invoke } from "@tauri-apps/api/core";
import { isDesktop } from "./ipc";
import {
  type Book,
  type Chapter,
  SCHEMA_VERSION,
  createBook,
  insertByKind,
  normalizeBook,
  schemaVersion,
  starterBook,
} from "./model/book";

export interface BookSummary {
  id: string;
  title: string;
  author: string;
  updatedAt: number;
  corrupt?: boolean;
}

export async function listBooks(): Promise<BookSummary[]> {
  if (!isDesktop) return [];
  return invoke<BookSummary[]>("list_books");
}

export async function loadBook(id: string): Promise<Book> {
  const contents = await invoke<string>("load_book", { id });
  let book: Book;
  try {
    book = JSON.parse(contents) as Book;
  } catch {
    throw new Error("the project file is corrupt or unreadable");
  }
  if (schemaVersion(book.schema) > SCHEMA_VERSION) {
    throw new Error("this project was made with a newer version of Margin; update the app to open it");
  }
  return book;
}

export async function saveBook(book: Book): Promise<void> {
  if (!isDesktop) return;
  await invoke("save_book", { id: book.id, contents: JSON.stringify(book, null, 2) });
}

export async function deleteBook(id: string): Promise<void> {
  if (!isDesktop) return;
  await invoke("delete_book", { id });
}

export async function moveChapterToBook(chapter: Chapter, targetId: string): Promise<void> {
  const target = normalizeBook(await loadBook(targetId));
  const moved = { ...chapter, content: structuredClone(chapter.content), updatedAt: Date.now() };
  await saveBook({ ...target, chapters: insertByKind(target.chapters, moved) });
}

const LAST_BOOK_KEY = "margin-last-book";

export function lastBookId(): string | null {
  return localStorage.getItem(LAST_BOOK_KEY);
}

export function rememberLastBook(id: string | null): void {
  if (id) localStorage.setItem(LAST_BOOK_KEY, id);
  else localStorage.removeItem(LAST_BOOK_KEY);
}

export function newBook(): Book {
  return createBook();
}

export function exampleBook(): Book {
  return starterBook();
}

export async function createAndOpenBook(
  open: (book: Book) => void,
  onError?: (message: string) => void,
): Promise<void> {
  const book = newBook();
  if (isDesktop) {
    try {
      await saveBook(book);
    } catch (e) {
      onError?.(`Could not create book: ${e}`);
    }
  }
  open(book);
}
