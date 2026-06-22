import { invoke } from "@tauri-apps/api/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { isDesktop } from "./ipc";
import type { ProofIssue } from "./editor/proofing";

export interface Issue {
  start: number;
  end: number;
  kind: "spelling" | "grammar";
  category: string;
  message: string;
  suggestions: string[];
}

export interface Segment {
  from: number;
  text: string;
  cpStart: number;
  cpLen: number;
}

function codepointLength(text: string): number {
  let n = 0;
  for (const _ of text) n++;
  return n;
}

function utf16Offset(text: string, codepoints: number): number {
  let units = 0;
  let count = 0;
  for (const ch of text) {
    if (count >= codepoints) break;
    units += ch.length;
    count++;
  }
  return units;
}

export function docText(doc: PMNode): { text: string; segments: Segment[] } {
  const segments: Segment[] = [];
  let text = "";
  let cp = 0;
  let first = true;
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    if (!first) {
      text += "\n\n";
      cp += 2;
    }
    first = false;
    node.forEach((child, offset) => {
      if (!child.isText) return;
      const t = child.text ?? "";
      const len = codepointLength(t);
      segments.push({ from: pos + 1 + offset, text: t, cpStart: cp, cpLen: len });
      text += t;
      cp += len;
    });
    return false;
  });
  return { text, segments };
}

export function mapOffset(segments: Segment[], cpOffset: number): number {
  for (const seg of segments) {
    if (cpOffset < seg.cpStart) return seg.from;
    if (cpOffset <= seg.cpStart + seg.cpLen) {
      return seg.from + utf16Offset(seg.text, cpOffset - seg.cpStart);
    }
  }
  const last = segments[segments.length - 1];
  return last ? last.from + last.text.length : 0;
}

export async function proofText(text: string, opts: { spelling: boolean; grammar: boolean }): Promise<Issue[]> {
  if (!isDesktop) return [];
  return invoke<Issue[]>("proof_text", { text, spelling: opts.spelling, grammar: opts.grammar });
}

export async function rememberWord(word: string): Promise<void> {
  if (!isDesktop) return;
  await invoke("remember_word", { word });
}

export function issueSignature(issue: { kind: string; word: string; message: string }): string {
  return `${issue.kind}|${issue.word}|${issue.message}`;
}

export async function runProof(
  doc: PMNode,
  opts: { spelling: boolean; grammar: boolean },
  ignored: Set<string>
): Promise<ProofIssue[]> {
  const { text, segments } = docText(doc);
  if (!text.trim()) return [];
  const issues = await proofText(text, opts);
  const chars = [...text];
  return issues
    .map((iss) => {
      const word = chars.slice(iss.start, iss.end).join("");
      return {
        from: mapOffset(segments, iss.start),
        to: mapOffset(segments, iss.end),
        kind: iss.kind,
        category: iss.category,
        message: iss.message,
        suggestions: iss.suggestions,
        word,
      };
    })
    .filter((i) => i.to > i.from && !ignored.has(issueSignature(i)));
}
