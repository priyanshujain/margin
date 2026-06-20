import { invoke } from "@tauri-apps/api/core";
import { isDesktop } from "./ipc";
import { type Book, createBook, starterBook } from "./model/book";

export interface BookSummary {
  id: string;
  title: string;
  author: string;
}

export async function listBooks(): Promise<BookSummary[]> {
  if (!isDesktop) return [];
  return invoke<BookSummary[]>("list_books");
}

export async function loadBook(id: string): Promise<Book> {
  const contents = await invoke<string>("load_book", { id });
  return JSON.parse(contents) as Book;
}

export async function saveBook(book: Book): Promise<void> {
  if (!isDesktop) return;
  await invoke("save_book", { id: book.id, contents: JSON.stringify(book, null, 2) });
}

export async function deleteBook(id: string): Promise<void> {
  if (!isDesktop) return;
  await invoke("delete_book", { id });
}

export function newBook(): Book {
  return createBook();
}

export function exampleBook(): Book {
  return starterBook();
}
