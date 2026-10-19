// The IPC the browser build answers with when there is no Tauri behind it: `pnpm dev` in a browser
// and the Playwright suite. Nothing here touches a disk.
import type { MockCall } from "@margin/core/ipc";
import { fixtureBooks } from "./fixture";

const files = new Map<string, string>();
for (const book of fixtureBooks()) files.set(book.id, JSON.stringify(book, null, 2));

const learned = new Set<string>();
const saves: { id: string; contents: string }[] = [];
let connectCancelled = 0;

declare global {
  interface Window {
    __marginMock?: { files: Map<string, string>; saves: typeof saves; cancelled: () => number };
  }
}

if (typeof window !== "undefined") {
  window.__marginMock = { files, saves, cancelled: () => connectCancelled };
}

function summary(id: string, contents: string) {
  try {
    const book = JSON.parse(contents);
    const updatedAt = Math.max(0, ...(book.chapters ?? []).map((c: { updatedAt?: number }) => c.updatedAt ?? 0));
    return { id, title: book.metadata?.title ?? "Untitled", author: book.metadata?.author ?? "", corrupt: false, updatedAt };
  } catch {
    return { id, title: "Unreadable project", author: "", corrupt: true, updatedAt: 0 };
  }
}

function misspellings(text: string) {
  const issues: { start: number; end: number; word: string; suggestions: string[] }[] = [];
  const chars = [...text];
  const joined = chars.join("");
  for (const match of joined.matchAll(/\bteh\b/gi)) {
    if (learned.has(match[0].toLowerCase())) continue;
    const start = [...joined.slice(0, match.index)].length;
    issues.push({ start, end: start + 3, word: match[0], suggestions: ["the", "ten"] });
  }
  return issues;
}

const OFFLINE = { connected: false, email: null, lastBackup: null, pending: false, needsReauth: false };

const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
  list_books: () => [...files].map(([id, contents]) => summary(id, contents)).sort((a, b) => b.updatedAt - a.updatedAt),
  load_book: ({ id }) => {
    const contents = files.get(String(id));
    if (contents === undefined) throw new Error("No such file or directory (os error 2)");
    return contents;
  },
  save_book: ({ id, contents }) => {
    files.set(String(id), String(contents));
    saves.push({ id: String(id), contents: String(contents) });
  },
  delete_book: ({ id }) => {
    files.delete(String(id));
  },
  spell_available: () => true,
  spell_check: ({ text }) => misspellings(String(text)),
  spell_learn: ({ word }) => {
    learned.add(String(word).toLowerCase());
  },
  spell_unlearn: ({ word }) => {
    learned.delete(String(word).toLowerCase());
  },
  grammar_available: () => true,
  grammar_check: () => [],
  gdrive_status: () => OFFLINE,
  gdrive_list_backups: () => [],
  gdrive_connect: () => "https://accounts.google.com/o/oauth2/auth?client_id=mock&state=mock",
  gdrive_connect_cancel: () => {
    connectCancelled += 1;
  },
  gdrive_disconnect: () => OFFLINE,
  list_system_fonts: () => [],
  update_channel: () => "none",
  appstore_latest: () => null,
  writing_available: () => false,
  run_writing_tool: () => undefined,
  log_note: () => undefined,
};

export const mockCall: MockCall = async <T>(command: string, args: Record<string, unknown> = {}) => {
  const handler = handlers[command];
  if (!handler) throw new Error(`${command} is not available in the browser`);
  return handler(args) as T;
};
