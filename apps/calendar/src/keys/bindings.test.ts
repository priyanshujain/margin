// The table's own invariants. Two bindings on one combo in one context would silently shadow each
// other, and a group the sheet does not render would silently hide a key, so both are asserted
// here rather than discovered later.

import { describe, expect, it } from "vitest";
import { allBindings, keyLabel, normalizeCombo } from "@margin/core/keys/bindings";
import { BINDINGS, GROUPS } from "./bindings";

describe("the binding table", () => {
  it("never binds one combo twice in the same context, suite keys included", () => {
    const seen = new Set<string>();
    for (const binding of allBindings()) {
      for (const key of binding.keys) {
        const slot = `${binding.context}:${normalizeCombo(key)}`;
        expect(seen.has(slot), `${slot} is bound twice`).toBe(false);
        seen.add(slot);
      }
    }
  });

  it("puts every binding in a group the sheet renders", () => {
    for (const binding of allBindings()) expect(GROUPS).toContain(binding.group);
  });

  it("names every binding", () => {
    for (const binding of BINDINGS) expect(binding.label).not.toBe("");
  });

  it("opens the palette on both suite combos", () => {
    const palette = allBindings().find((b) => b.command === "command-palette");
    expect(palette?.keys).toEqual(expect.arrayContaining(["cmd+k", "cmd+shift+p"]));
  });

  it("documents Escape without claiming to handle it", () => {
    const escape = allBindings().find((b) => b.keys.includes("Escape"));
    expect(escape?.command).toBeNull();
  });

  it("prints a shifted letter as a shifted letter", () => {
    expect(keyLabel("H")).toBe("⇧H");
    expect(keyLabel("Enter")).toBe("↩");
    expect(keyLabel("Escape")).toBe("⎋");
  });
});
