export interface ChapterPosition {
  from: number;
  to: number;
  scroll: number;
}

const KEY = "margin-positions";

type Store = Record<string, Record<string, ChapterPosition>>;

function readAll(): Store {
  try {
    return JSON.parse(localStorage.getItem(KEY) || "{}") as Store;
  } catch {
    return {};
  }
}

export function loadPosition(bookId: string, chapterId: string): ChapterPosition | null {
  return readAll()[bookId]?.[chapterId] ?? null;
}

export function savePosition(bookId: string, chapterId: string, position: ChapterPosition): void {
  const all = readAll();
  (all[bookId] ||= {})[chapterId] = position;
  localStorage.setItem(KEY, JSON.stringify(all));
}

const ACTIVE_KEY = "margin-active-chapter";

function readActive(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem(ACTIVE_KEY) || "{}") as Record<string, string>;
  } catch {
    return {};
  }
}

export function loadActiveChapter(bookId: string): string | null {
  return readActive()[bookId] ?? null;
}

export function saveActiveChapter(bookId: string, chapterId: string): void {
  const all = readActive();
  all[bookId] = chapterId;
  localStorage.setItem(ACTIVE_KEY, JSON.stringify(all));
}
