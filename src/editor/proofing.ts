import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { Node as PMNode } from "@tiptap/pm/model";

export interface ProofIssue {
  from: number;
  to: number;
  kind: "spelling" | "grammar";
  category: string;
  message: string;
  suggestions: string[];
  word: string;
}

export type ProofSeverity = "error" | "warn" | "suggest";

const WARN_CATEGORIES = new Set(["Redundancy", "Repetition", "Formatting", "Regionalism"]);
const SUGGEST_CATEGORIES = new Set(["Style", "Readability", "Enhancement", "Miscellaneous"]);

export function severityFor(category: string): ProofSeverity {
  if (WARN_CATEGORIES.has(category)) return "warn";
  if (SUGGEST_CATEGORIES.has(category)) return "suggest";
  return "error";
}

export interface ProofCoords {
  left: number;
  top: number;
  bottom: number;
}

interface ProofingState {
  issues: ProofIssue[];
  decorations: DecorationSet;
}

export const proofingKey = new PluginKey<ProofingState>("proofing");

function buildDecorations(doc: PMNode, issues: ProofIssue[], cursor: number | null): DecorationSet {
  const size = doc.content.size;
  const decos = issues
    .filter((i) => i.from < i.to && i.to <= size)
    .filter((i) => cursor === null || cursor < i.from || cursor > i.to)
    .map((i) => Decoration.inline(i.from, i.to, { class: `proof-mark sev-${severityFor(i.category)}` }));
  return DecorationSet.create(doc, decos);
}

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    proofing: {
      setProofIssues: (issues: ProofIssue[]) => ReturnType;
      clearProofIssues: () => ReturnType;
    };
  }
}

export interface ProofingStorage {
  onClickIssue: ((issue: ProofIssue, coords: ProofCoords) => void) | null;
}

export const Proofing = Extension.create<unknown, ProofingStorage>({
  name: "proofing",

  addStorage() {
    return { onClickIssue: null };
  },

  addProseMirrorPlugins() {
    const storage = this.storage;
    return [
      new Plugin<ProofingState>({
        key: proofingKey,
        state: {
          init: () => ({ issues: [], decorations: DecorationSet.empty }),
          apply(tr, value, _old, newState) {
            const sel = newState.selection;
            const cursor = sel.empty ? sel.head : null;
            const meta = tr.getMeta(proofingKey) as { issues: ProofIssue[] } | { clear: true } | undefined;
            if (meta && "issues" in meta) {
              return { issues: meta.issues, decorations: buildDecorations(newState.doc, meta.issues, cursor) };
            }
            if (meta && "clear" in meta) {
              return { issues: [], decorations: DecorationSet.empty };
            }
            if (tr.docChanged && value.issues.length) {
              const issues = value.issues
                .map((i) => ({ ...i, from: tr.mapping.map(i.from), to: tr.mapping.map(i.to) }))
                .filter((i) => i.to > i.from);
              return { issues, decorations: buildDecorations(newState.doc, issues, cursor) };
            }
            if (tr.selectionSet && value.issues.length) {
              return { issues: value.issues, decorations: buildDecorations(newState.doc, value.issues, cursor) };
            }
            return value;
          },
        },
        props: {
          decorations(state) {
            return proofingKey.getState(state)?.decorations ?? DecorationSet.empty;
          },
          handleClick(view, pos) {
            const s = proofingKey.getState(view.state);
            if (!s || !s.issues.length) return false;
            const hits = s.issues.filter((i) => pos >= i.from && pos <= i.to);
            if (!hits.length) return false;
            const issue = hits.find((i) => i.kind === "spelling") ?? hits[0];
            if (!storage.onClickIssue) return false;
            const start = view.coordsAtPos(issue.from);
            const end = view.coordsAtPos(issue.to);
            storage.onClickIssue(issue, {
              left: (start.left + end.left) / 2,
              top: start.top,
              bottom: end.bottom,
            });
            return false;
          },
        },
      }),
    ];
  },

  addCommands() {
    return {
      setProofIssues:
        (issues) =>
        ({ tr, dispatch }) => {
          if (dispatch) dispatch(tr.setMeta(proofingKey, { issues }));
          return true;
        },
      clearProofIssues:
        () =>
        ({ tr, dispatch }) => {
          if (dispatch) dispatch(tr.setMeta(proofingKey, { clear: true }));
          return true;
        },
    };
  },
});
