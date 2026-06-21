import { create } from "zustand";
import type { JSONContent } from "@tiptap/core";
import {
  type Book,
  type BookMetadata,
  type BookSettings,
  type ChapterKind,
  type Cover,
  chapterKind,
  createChapter,
  createPage,
  normalizeBook,
} from "../model/book";

export const COVER_ID = "__cover__";

interface BookState {
  book: Book | null;
  activeChapterId: string;
  dirty: boolean;
  exporting: string | null;
  notice: string | null;
  setExporting: (label: string | null) => void;
  setNotice: (message: string | null) => void;
  openBook: (book: Book) => void;
  closeBook: () => void;
  setActiveChapter: (id: string) => void;
  setChapterContent: (id: string, content: JSONContent) => void;
  setChapterTitle: (id: string, title: string) => void;
  addChapter: () => void;
  addPage: (group: "front" | "back", title: string) => void;
  deleteChapter: (id: string) => void;
  moveChapter: (from: number, to: number, toKind?: ChapterKind) => void;
  setMetadata: (patch: Partial<BookMetadata>) => void;
  setSettings: (patch: Partial<BookSettings>) => void;
  setCover: (patch: Partial<Cover>) => void;
  markSaved: () => void;
}

export const useBook = create<BookState>((set) => ({
  book: null,
  activeChapterId: "",
  dirty: false,
  exporting: null,
  notice: null,
  setExporting: (label) => set({ exporting: label }),
  setNotice: (message) => set({ notice: message }),
  openBook: (book) => {
    const normalized = normalizeBook(book);
    set({ book: normalized, activeChapterId: normalized.chapters[0]?.id ?? "", dirty: false });
  },
  closeBook: () => set({ book: null, activeChapterId: "", dirty: false }),
  setActiveChapter: (id) => set({ activeChapterId: id }),
  setChapterContent: (id, content) =>
    set((s) =>
      s.book
        ? { dirty: true, book: { ...s.book, chapters: s.book.chapters.map((c) => (c.id === id ? { ...c, content, updatedAt: Date.now() } : c)) } }
        : {}
    ),
  setChapterTitle: (id, title) =>
    set((s) =>
      s.book
        ? { dirty: true, book: { ...s.book, chapters: s.book.chapters.map((c) => (c.id === id ? { ...c, title, updatedAt: Date.now() } : c)) } }
        : {}
    ),
  addChapter: () =>
    set((s) => {
      if (!s.book) return {};
      const chapter = createChapter();
      const chapters = [...s.book.chapters];
      const backCount = chapters.filter((c) => chapterKind(c) === "back").length;
      chapters.splice(chapters.length - backCount, 0, chapter);
      return { activeChapterId: chapter.id, dirty: true, book: { ...s.book, chapters } };
    }),
  addPage: (group, title) =>
    set((s) => {
      if (!s.book) return {};
      const page = createPage(group, title);
      const chapters = [...s.book.chapters];
      const insertAt =
        group === "front" ? chapters.filter((c) => chapterKind(c) === "front").length : chapters.length;
      chapters.splice(insertAt, 0, page);
      return { activeChapterId: page.id, dirty: true, book: { ...s.book, chapters } };
    }),
  deleteChapter: (id) =>
    set((s) => {
      if (!s.book) return {};
      const index = s.book.chapters.findIndex((c) => c.id === id);
      if (index === -1) return {};
      const chapters = s.book.chapters.filter((c) => c.id !== id);
      const activeChapterId =
        s.activeChapterId === id ? (chapters[index] ?? chapters[index - 1])?.id ?? COVER_ID : s.activeChapterId;
      return { dirty: true, activeChapterId, book: { ...s.book, chapters } };
    }),
  moveChapter: (from, to, toKind) =>
    set((s) => {
      if (!s.book) return {};
      const chapters = [...s.book.chapters];
      if (from < 0 || from >= chapters.length) return {};
      const target = chapters[from];
      const nextKind = toKind ?? chapterKind(target);
      const dest = Math.max(0, Math.min(from < to ? to - 1 : to, chapters.length - 1));
      if (from === dest && chapterKind(target) === nextKind) return {};
      const [moved] = chapters.splice(from, 1);
      chapters.splice(dest, 0, nextKind === "body" ? { ...moved, kind: undefined } : { ...moved, kind: nextKind });
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
