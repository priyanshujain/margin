import { useEffect, useState } from "react";
import { ShortcutsSheet, registerCommands, useKeyContext } from "@margin/core/keys";
import { GROUPS, SHORTCUTS } from "../keys/bindings";

export function Shortcuts() {
  const [open, setOpen] = useState(false);

  useEffect(() => registerCommands({ [SHORTCUTS]: () => setOpen((wasOpen) => !wasOpen) }), []);
  useKeyContext("overlay", open);

  return <ShortcutsSheet open={open} onClose={() => setOpen(false)} groups={GROUPS} />;
}
