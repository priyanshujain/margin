import { create } from "zustand";
import type { JSONContent } from "@tiptap/core";
import { type Book, type BookMetadata, type BookSettings, createChapter } from "../model/book";

interface BookState {
  book: Book | null;
  activeChapterId: string;
  dirty: boolean;
  openBook: (book: Book) => void;
  closeBook: () => void;
  setActiveChapter: (id: string) => void;
  setChapterContent: (id: string, content: JSONContent) => void;
  setChapterTitle: (id: string, title: string) => void;
  addChapter: () => void;
  setMetadata: (patch: Partial<BookMetadata>) => void;
  setSettings: (patch: Partial<BookSettings>) => void;
  markSaved: () => void;
}

export const useBook = create<BookState>((set) => ({
  book: null,
  activeChapterId: "",
  dirty: false,
  openBook: (book) => set({ book, activeChapterId: book.chapters[0]?.id ?? "", dirty: false }),
  closeBook: () => set({ book: null, activeChapterId: "", dirty: false }),
  setActiveChapter: (id) => set({ activeChapterId: id }),
  setChapterContent: (id, content) =>
    set((s) =>
      s.book
        ? { dirty: true, book: { ...s.book, chapters: s.book.chapters.map((c) => (c.id === id ? { ...c, content } : c)) } }
        : {}
    ),
  setChapterTitle: (id, title) =>
    set((s) =>
      s.book
        ? { dirty: true, book: { ...s.book, chapters: s.book.chapters.map((c) => (c.id === id ? { ...c, title } : c)) } }
        : {}
    ),
  addChapter: () =>
    set((s) => {
      if (!s.book) return {};
      const chapter = createChapter();
      return { activeChapterId: chapter.id, dirty: true, book: { ...s.book, chapters: [...s.book.chapters, chapter] } };
    }),
  setMetadata: (patch) =>
    set((s) => (s.book ? { dirty: true, book: { ...s.book, metadata: { ...s.book.metadata, ...patch } } } : {})),
  setSettings: (patch) =>
    set((s) => (s.book ? { dirty: true, book: { ...s.book, settings: { ...s.book.settings, ...patch } } } : {})),
  markSaved: () => set({ dirty: false }),
}));
