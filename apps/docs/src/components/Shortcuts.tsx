import { useEffect, useState } from "react";
import { ShortcutsSheet, useKeyContext } from "@margin/core/keys";
import { GROUPS } from "../keys/bindings";
import { onCommand } from "../keys/commands";

export function Shortcuts() {
  const [open, setOpen] = useState(false);

  useEffect(() => onCommand("shortcuts", () => setOpen((v) => !v)), []);
  useKeyContext("overlay", open);

  return <ShortcutsSheet open={open} onClose={() => setOpen(false)} groups={GROUPS} />;
}
