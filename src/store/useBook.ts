import { create } from "zustand";
import type { JSONContent } from "@tiptap/core";
import {
  type Book,
  type BookMetadata,
  type BookSettings,
  type Cover,
  createChapter,
  normalizeBook,
} from "../model/book";

export const COVER_ID = "__cover__";

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
  moveChapter: (from: number, to: number) => void;
  setMetadata: (patch: Partial<BookMetadata>) => void;
  setSettings: (patch: Partial<BookSettings>) => void;
  setCover: (patch: Partial<Cover>) => void;
  markSaved: () => void;
}

export const useBook = create<BookState>((set) => ({
  book: null,
  activeChapterId: "",
  dirty: false,
  openBook: (book) => {
    const normalized = normalizeBook(book);
    set({ book: normalized, activeChapterId: normalized.chapters[0]?.id ?? "", dirty: false });
  },
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
  moveChapter: (from, to) =>
    set((s) => {
      if (!s.book || from === to) return {};
      const chapters = [...s.book.chapters];
      if (from < 0 || from >= chapters.length || to < 0 || to >= chapters.length) return {};
      const [moved] = chapters.splice(from, 1);
      chapters.splice(to, 0, moved);
      return { dirty: true, book: { ...s.book, chapters } };
    }),
  setMetadata: (patch) =>
    set((s) => (s.book ? { dirty: true, book: { ...s.book, metadata: { ...s.book.metadata, ...patch } } } : {})),
  setSettings: (patch) =>
    set((s) => (s.book ? { dirty: true, book: { ...s.book, settings: { ...s.book.settings, ...patch } } } : {})),
  setCover: (patch) =>
    set((s) => (s.book ? { dirty: true, book: { ...s.book, cover: { ...s.book.cover, ...patch } } } : {})),
  markSaved: () => set({ dirty: false }),
}));
