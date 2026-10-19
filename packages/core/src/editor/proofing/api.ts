// The proofing commands margin-proof exposes to the webview. Offsets are characters counted from the
// start of the run of text checked; the caller adds its own base offset.

export interface SpellIssue {
  start: number;
  end: number;
  word: string;
  suggestions: string[];
}

export interface GrammarIssue {
  start: number;
  end: number;
  kind: string;
  message: string;
  suggestions: string[];
}

export type ProofCall = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export interface ProofApi {
  spellCheck: (text: string) => Promise<SpellIssue[]>;
  spellLearn: (word: string) => Promise<void>;
  spellUnlearn: (word: string) => Promise<void>;
  spellAvailable: () => Promise<boolean>;
  grammarCheck: (text: string) => Promise<GrammarIssue[]>;
  grammarAvailable: () => Promise<boolean>;
}

export function proofApi(call: ProofCall): ProofApi {
  return {
    spellCheck: (text) => call<SpellIssue[]>("spell_check", { text }),
    spellLearn: (word) => call<void>("spell_learn", { word }),
    spellUnlearn: (word) => call<void>("spell_unlearn", { word }),
    spellAvailable: () => call<boolean>("spell_available"),
    grammarCheck: (text) => call<GrammarIssue[]>("grammar_check", { text }),
    grammarAvailable: () => call<boolean>("grammar_available"),
  };
}
