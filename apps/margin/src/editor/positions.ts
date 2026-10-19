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

function write(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    return;
  }
}

export function loadPosition(bookId: string, chapterId: string): ChapterPosition | null {
  return readAll()[bookId]?.[chapterId] ?? null;
}

export function savePosition(bookId: string, chapterId: string, position: ChapterPosition): void {
  const all = readAll();
  (all[bookId] ||= {})[chapterId] = position;
  write(KEY, all);
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
  write(ACTIVE_KEY, all);
}

export function clearPositions(bookId: string): void {
  const positions = readAll();
  const active = readActive();
  delete positions[bookId];
  delete active[bookId];
  write(KEY, positions);
  write(ACTIVE_KEY, active);
}
