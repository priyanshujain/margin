import { create } from "zustand";
import type { JSONContent } from "@tiptap/core";
import { type Book, createChapter, starterBook } from "../model/book";

interface BookState {
  book: Book;
  activeChapterId: string;
  path: string | null;
  dirty: boolean;
  setActiveChapter: (id: string) => void;
  setChapterContent: (id: string, content: JSONContent) => void;
  setChapterTitle: (id: string, title: string) => void;
  addChapter: () => void;
  replaceBook: (book: Book, path: string) => void;
  markSaved: (path: string) => void;
}

export const useBook = create<BookState>((set) => {
  const book = starterBook();
  return {
    book,
    activeChapterId: book.chapters[0].id,
    path: null,
    dirty: false,
    setActiveChapter: (id) => set({ activeChapterId: id }),
    setChapterContent: (id, content) =>
      set((s) => ({
        dirty: true,
        book: {
          ...s.book,
          chapters: s.book.chapters.map((c) => (c.id === id ? { ...c, content } : c)),
        },
      })),
    setChapterTitle: (id, title) =>
      set((s) => ({
        dirty: true,
        book: {
          ...s.book,
          chapters: s.book.chapters.map((c) => (c.id === id ? { ...c, title } : c)),
        },
      })),
    addChapter: () =>
      set((s) => {
        const chapter = createChapter();
        return {
          activeChapterId: chapter.id,
          dirty: true,
          book: { ...s.book, chapters: [...s.book.chapters, chapter] },
        };
      }),
    replaceBook: (book, path) =>
      set({ book, activeChapterId: book.chapters[0]?.id ?? "", path, dirty: false }),
    markSaved: (path) => set({ path, dirty: false }),
  };
});
