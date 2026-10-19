import { configureProofing, proofApi, useProofing } from "@margin/core/editor/base";
import { call } from "../ipc";
import { useBook } from "./useBook";

const PREFIX = "margin";

function proofsThisBook(): boolean {
  return (useBook.getState().book?.metadata.language ?? "en") === "en";
}

const api = proofApi(call);

configureProofing({
  prefix: PREFIX,
  api: {
    ...api,
    spellCheck: (text) => (proofsThisBook() ? api.spellCheck(text) : Promise.resolve([])),
    grammarCheck: (text) => (proofsThisBook() ? api.grammarCheck(text) : Promise.resolve([])),
  },
});

if (typeof localStorage !== "undefined" && localStorage.getItem(`${PREFIX}-grammar`) === null) {
  useProofing.setState({ grammar: false });
}

export { useProofing };
