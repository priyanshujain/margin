import type { CommandId } from "./bindings";

export type Handler = () => void;

const handlers = new Map<CommandId, Handler[]>();

export function registerCommands(map: Partial<Record<CommandId, Handler>>): () => void {
  const added: [CommandId, Handler][] = [];
  for (const [id, handler] of Object.entries(map) as [CommandId, Handler | undefined][]) {
    if (!handler) continue;
    const stack = handlers.get(id) ?? [];
    stack.push(handler);
    handlers.set(id, stack);
    added.push([id, handler]);
  }
  return () => {
    for (const [id, handler] of added) {
      const stack = handlers.get(id);
      if (!stack) continue;
      const at = stack.lastIndexOf(handler);
      if (at !== -1) stack.splice(at, 1);
      if (stack.length === 0) handlers.delete(id);
    }
  };
}

export function isRegistered(id: CommandId): boolean {
  return (handlers.get(id)?.length ?? 0) > 0;
}

export function runCommand(id: CommandId): void {
  const stack = handlers.get(id);
  if (!stack || stack.length === 0) return;
  stack[stack.length - 1]();
}

export function commandMatches(label: string, query: string): boolean {
  const needle = query.toLowerCase().replace(/\s+/g, "");
  if (!needle) return true;
  const hay = label.toLowerCase();
  let at = 0;
  for (const ch of needle) {
    at = hay.indexOf(ch, at);
    if (at === -1) return false;
    at += 1;
  }
  return true;
}
