// Spelling and grammar state lives in margin-core's editor layer; this wires it to Docs' IPC and
// storage keys (margindocs-spelling, margindocs-grammar) before anything reads it.

import { configureProofing, proofApi, useProofing } from "@margin/core/editor";
import { call } from "../ipc";

configureProofing({ prefix: "margindocs", api: proofApi(call) });

export { useProofing };
export type { ProofTarget, SpellAvailability } from "@margin/core/editor";
