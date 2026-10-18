import { describe, expect, it } from "vitest";
import {
  allBindings,
  groupsOf,
  keyLabel,
  keysFor,
  normalizeCombo,
  paletteCommands,
  registerBindings,
  suiteBindings,
  type Binding,
} from "./bindings";

const APP: Binding[] = [
  { keys: ["cmd+a"], command: "select-all", label: "Select all", context: "view", group: "Edit" },
  {
    keys: ["cmd+shift+a"],
    command: "attach",
    label: "Attach a file",
    context: "editor",
    group: "Edit",
    allowInInput: true,
  },
];

describe("the suite binding table", () => {
  const table = [...suiteBindings({ quickOpen: true }), ...APP];

  it("never binds one combo twice in the same context", () => {
    const seen = new Set<string>();
    for (const binding of table) {
      for (const key of binding.keys) {
        const slot = `${binding.context}:${normalizeCombo(key)}`;
        expect(seen.has(slot), `${slot} is bound twice`).toBe(false);
        seen.add(slot);
      }
    }
  });

  it("gives every binding a label, including the ones it does not own", () => {
    for (const binding of table) expect(binding.label).not.toBe("");
  });

  it("documents Escape without claiming to handle it", () => {
    const escape = table.find((b) => b.keys.includes("Escape"));
    expect(escape?.command).toBeNull();
  });

  it("keeps the palette reachable from a text field on both combos", () => {
    const palette = table.find((b) => b.command === "command-palette");
    expect(palette?.allowInInput).toBe(true);
    expect(palette?.context).toBe("global");
    expect(palette?.keys.map(normalizeCombo)).toEqual(["cmd+k", "cmd+shift+p"]);
  });

  it("binds quick open only when the app has one", () => {
    expect(suiteBindings().some((b) => b.command === "quick-open")).toBe(false);
    expect(suiteBindings({ quickOpen: true }).some((b) => b.command === "quick-open")).toBe(true);
  });
});

describe("the registry", () => {
  it("adds an app's table to the suite set and takes it away again", () => {
    const stopSuite = registerBindings(suiteBindings());
    const stopApp = registerBindings(APP);
    expect(groupsOf()).toEqual(["App", "Edit"]);
    expect(keysFor("select-all")).toEqual(["cmd+a"]);
    expect(paletteCommands().map((b) => b.command)).toContain("settings");
    stopApp();
    expect(keysFor("select-all")).toEqual([]);
    stopSuite();
    expect(allBindings()).toEqual([]);
  });
});

describe("combos", () => {
  it("keeps Cmd+A and Cmd+Shift+A apart", () => {
    expect(normalizeCombo("cmd+a")).not.toBe(normalizeCombo("cmd+shift+a"));
  });

  it("reads a combo the same way the dispatcher builds one", () => {
    expect(normalizeCombo("Cmd+K")).toBe("cmd+k");
    expect(normalizeCombo("cmd+k")).toBe("cmd+k");
    expect(normalizeCombo("H")).toBe("shift+h");
    expect(normalizeCombo("shift+h")).toBe("shift+h");
    expect(normalizeCombo("/")).toBe("/");
    expect(normalizeCombo("shift+Tab")).toBe("shift+Tab");
    expect(normalizeCombo("shift+ ")).toBe("shift+ ");
  });

  it("prints a key the way a keycap does", () => {
    expect(keyLabel("shift+h")).toBe("⇧H");
    expect(keyLabel("h")).toBe("h".toUpperCase());
    expect(keyLabel("Enter")).toBe("↩");
    expect(keyLabel("Escape")).toBe("⎋");
    expect(keyLabel(" ")).toBe("Space");
  });
});
