import { create } from "zustand";

interface ProofingState {
  spelling: boolean;
  grammar: boolean;
  ignored: Set<string>;
  toggleSpelling: () => void;
  toggleGrammar: () => void;
  ignore: (signature: string) => void;
  resetIgnored: () => void;
}

export const useProofing = create<ProofingState>((set) => ({
  spelling: true,
  grammar: false,
  ignored: new Set(),
  toggleSpelling: () => set((s) => ({ spelling: !s.spelling })),
  toggleGrammar: () => set((s) => ({ grammar: !s.grammar })),
  ignore: (signature) =>
    set((s) => {
      const ignored = new Set(s.ignored);
      ignored.add(signature);
      return { ignored };
    }),
  resetIgnored: () => set({ ignored: new Set() }),
}));
