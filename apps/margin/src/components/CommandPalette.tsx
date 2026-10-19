import { useEffect, useState } from "react";
import { CommandPalette as CorePalette, registerCommands, useKeyContext } from "@margin/core/keys";
import { COMMAND_PALETTE } from "../keys/bindings";

export function CommandPalette() {
  const [open, setOpen] = useState(false);

  useEffect(() => registerCommands({ [COMMAND_PALETTE]: () => setOpen((wasOpen) => !wasOpen) }), []);
  useKeyContext("overlay", open);

  return <CorePalette open={open} onClose={() => setOpen(false)} empty="No command by that name." />;
}
