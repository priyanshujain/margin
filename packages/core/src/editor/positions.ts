// Where the caret and the scroller were, per document, so reopening a file lands where you left
// it rather than at the top.
//
// Margin keys this by book and then by chapter because a book is a folder of many small documents
// that are read as one. Here there is no such nesting: a document is a file, a file has an
// absolute path, and that path is the whole key. Two roots holding a same-named file are two
// different paths and two different entries, which is the point.

export interface DocumentPosition {
  from: number;
  to: number;
  scroll: number;
}

/** Old entries fall off the end rather than growing a map nobody ever prunes. */
const LIMIT = 200;

type Store = Record<string, DocumentPosition>;

function readAll(key: string): Store {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as Store) : {};
  } catch {
    return {};
  }
}

function writeAll(key: string, store: Store): void {
  try {
    localStorage.setItem(key, JSON.stringify(store));
  } catch {
    return;
  }
}

export interface PositionStore {
  load: (path: string) => DocumentPosition | null;
  save: (path: string, position: DocumentPosition) => void;
}

/** Caret and scroll per document path, kept in localStorage under `key`. */
export function positionStore(key: string): PositionStore {
  return {
    load: (path) => readAll(key)[path] ?? null,
    save: (path, position) => {
      const store = readAll(key);
      delete store[path];
      store[path] = position;
      const paths = Object.keys(store);
      for (const stale of paths.slice(0, Math.max(0, paths.length - LIMIT))) delete store[stale];
      writeAll(key, store);
    },
  };
}
