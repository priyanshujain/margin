import { Editor as TiptapEditor } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import type { EditorState } from "@tiptap/pm/state";
import { editorExtensions } from "./extensions";

export interface ChapterState {
  state: EditorState;
  content: JSONContent;
  scroll: number;
}

export interface EditorHooks {
  onUpdate: (content: JSONContent) => void;
  onContentError: (error: Error) => void;
}

const SCROLL_KEEPOUT = { top: 32, right: 0, bottom: 96, left: 0 };
const MAX_STATES = 24;

const states = new Map<string, ChapterState>();
const stateKey = (bookId: string, chapterId: string) => `${bookId}:${chapterId}`;

let instance: TiptapEditor | null = null;
let hooks: EditorHooks | null = null;

export function sharedEditor(next: EditorHooks): TiptapEditor {
  hooks = next;
  if (!instance) {
    instance = new TiptapEditor({
      extensions: editorExtensions,
      enableContentCheck: true,
      editorProps: {
        attributes: { class: "prose" },
        scrollThreshold: SCROLL_KEEPOUT,
        scrollMargin: SCROLL_KEEPOUT,
      },
      onContentError: ({ error }) => hooks?.onContentError(error),
      onUpdate: ({ editor }) => hooks?.onUpdate(editor.getJSON()),
    });
  }
  return instance;
}

export function loadChapterState(bookId: string, chapterId: string): ChapterState | undefined {
  return states.get(stateKey(bookId, chapterId));
}

export function saveChapterState(bookId: string, chapterId: string, entry: ChapterState): void {
  for (const id of states.keys()) {
    if (!id.startsWith(`${bookId}:`)) states.delete(id);
  }
  const id = stateKey(bookId, chapterId);
  states.delete(id);
  states.set(id, entry);
  for (const oldest of states.keys()) {
    if (states.size <= MAX_STATES) break;
    states.delete(oldest);
  }
}

export function clearChapterStates(bookId: string): void {
  for (const id of states.keys()) {
    if (id.startsWith(`${bookId}:`)) states.delete(id);
  }
}
