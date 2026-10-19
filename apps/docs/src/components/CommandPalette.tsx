import { useEffect, useState } from "react";
import { CommandPalette as CorePalette, useKeyContext } from "@margin/core/keys";
import { onCommand } from "../keys/commands";

export function CommandPalette() {
  const [open, setOpen] = useState(false);

  useEffect(() => onCommand("command-palette", () => setOpen((wasOpen) => !wasOpen)), []);
  useKeyContext("overlay", open);

  return (
    <CorePalette open={open} onClose={() => setOpen(false)} empty="No command by that name." />
  );
}
