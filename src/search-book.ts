import type { JSONContent } from "@tiptap/core";
import { buildRegex, type SearchOptions } from "./editor/search";

function collectRuns(node: JSONContent, out: string[]) {
  const children = node.content;
  if (!children) return;
  let run = "";
  for (const child of children) {
    if (child.type === "text") {
      run += child.text ?? "";
    } else {
      if (run) {
        out.push(run);
        run = "";
      }
      collectRuns(child, out);
    }
  }
  if (run) out.push(run);
}

export function countMatches(content: JSONContent, regex: RegExp): number {
  const runs: string[] = [];
  collectRuns(content, runs);
  let count = 0;
  for (const run of runs) {
    regex.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = regex.exec(run)) !== null) {
      count++;
      if (m.index === regex.lastIndex) regex.lastIndex++;
    }
  }
  return count;
}

export function replaceInContent(
  content: JSONContent,
  regex: RegExp,
  replacement: string
): { content: JSONContent; count: number } {
  let count = 0;
  const walk = (node: JSONContent): JSONContent => {
    if (node.type === "text" && node.text) {
      regex.lastIndex = 0;
      const text = node.text.replace(regex, () => {
        count++;
        return replacement;
      });
      return text === node.text ? node : { ...node, text };
    }
    if (node.content) return { ...node, content: node.content.map(walk) };
    return node;
  };
  const next = walk(content);
  return { content: next, count };
}

export function bookRegex(query: string, options: SearchOptions): RegExp | null {
  return buildRegex(query, options);
}
