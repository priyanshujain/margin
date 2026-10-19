export { createEngine } from "./engine";
export type { EngineConfig } from "./engine";

export { SearchHighlight, buildRegex, findMatches, searchKey, searchStateOf } from "./search";
export type { SearchMatch, SearchOptions, SearchState } from "./search";
export * from "./plainFind";

export { FindBar } from "./FindBar";
export type { DocumentFind, FindBarProps, FindOptions, FindState } from "./FindBar";

export { positionStore } from "./positions";
export type { DocumentPosition, PositionStore } from "./positions";

export * from "./fits";
export { isolatingLine } from "./lines";


export { proofApi } from "./proofing/api";
export type { GrammarIssue, ProofApi, ProofCall, SpellIssue } from "./proofing/api";
export { configureProofing, proofing, useProofing } from "./proofing/store";
export type { ProofTarget, ProofingConfig, SpellAvailability } from "./proofing/store";
export { Proofing, openSpellingMenu, replaceSpelling } from "./proofing/extension";
export { ProofPopover } from "./ProofPopover";
export type { ProofPopoverProps } from "./ProofPopover";

export { runWritingTool, writingBridge } from "./writing";
export type { WritingBridge, WritingTool } from "./writing";

export { sanitize as typstSanitize, str as typstString } from "./typst";

export { TOOLBAR_ITEMS, TOOL_ICONS, tool } from "./toolbar";
export type { ToolIcon, ToolbarItem } from "./toolbar";
