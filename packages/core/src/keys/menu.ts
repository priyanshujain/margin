import type { CommandId } from "./bindings";
import { runCommand } from "./commands";

export function createMenuHandler(ids: readonly CommandId[]): (id: string) => void {
  const known = new Set<string>(ids);
  return (id) => {
    if (known.has(id)) runCommand(id);
  };
}
