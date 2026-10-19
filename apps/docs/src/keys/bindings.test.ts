// The table's own invariants. Two bindings on one combo in one context would silently shadow each
// other, a group the sheet does not render would silently hide a key, and a menu id with no real
// command behind it would build fine in Rust and do nothing at all in TypeScript, so all three are
// asserted here rather than discovered later.

import { describe, expect, it } from "vitest";
import { BINDINGS, GROUPS, keysFor, normalizeCombo } from "./bindings";
import { COMMANDS } from "./commands";
import { MENU_IDS } from "./menu";

describe("the binding table", () => {
  it("never binds one combo twice in the same context", () => {
    const seen = new Set<string>();
    for (const binding of BINDINGS) {
      for (const key of binding.keys) {
        const slot = `${binding.context}:${normalizeCombo(key)}`;
        expect(seen.has(slot), `${slot} is bound twice`).toBe(false);
        seen.add(slot);
      }
    }
  });

  it("puts every binding in a group the sheet renders", () => {
    for (const binding of BINDINGS) expect(GROUPS).toContain(binding.group);
  });

  it("can name every binding, including the ones it does not own", () => {
    for (const binding of BINDINGS) {
      expect(binding.label).not.toBe("");
    }
  });

  it("documents Escape without claiming to handle it", () => {
    const escape = BINDINGS.find((b) => b.keys.includes("Escape"));
    expect(escape?.command).toBeNull();
  });
});

// Google Docs' own chords, checked against the table rather than against a comment. Somebody
// moving off Docs presses these before they read anything, and the two that moved are the two that
// used to be somewhere else, so a revert of either is a regression nobody would notice by eye.
describe("the chords a Google Docs user arrives with", () => {
  const bindingFor = (combo: string) =>
    BINDINGS.find((b) => b.keys.some((key) => normalizeCombo(key) === combo));

  it("gives Cmd+Shift+K to the link tool, since Cmd+K is the suite's palette", () => {
    const binding = bindingFor("cmd+shift+k");
    expect(binding?.command).toBe("insert-link");
    // View rather than global: with the palette or settings on screen there is no selection to
    // put an address on, and the context stack is what says so.
    expect(binding?.context).toBe("view");
    expect(binding?.allowInInput).toBe(true);
  });

  it("opens the command palette on Cmd+K and Cmd+Shift+P and leaves Cmd+P on quick open", () => {
    expect(keysFor("command-palette")).toEqual(["cmd+k", "cmd+shift+p"]);
    expect(keysFor("quick-open")).toEqual(["cmd+p"]);
    expect(bindingFor("cmd+k")?.command).toBe("command-palette");
    expect(bindingFor("cmd+shift+p")?.command).toBe("command-palette");
    expect(bindingFor("cmd+p")?.command).toBe("quick-open");
  });

  it("answers Cmd+U rather than leaving the webview to underline something markdown cannot spell", () => {
    // A note binding would document the key without taking it, and an untaken Cmd+U inside a
    // contenteditable is WebKit wrapping the selection in a tag the schema has no room for. This
    // one has a command, which is what makes the keymap call preventDefault on it.
    const binding = bindingFor("cmd+u");
    expect(binding?.command).toBe("underline-unsupported");
    expect(binding?.context).toBe("view");
  });

  it("keeps the app's own chords where the rest of the app put them", () => {
    // Google Docs spends Cmd+\ on clear formatting and Cmd+Shift+E on centring, and both of those
    // stay this app's. The complaint was about editing keys, and moving the sidebar or the export
    // would cost muscle memory nobody offered up.
    expect(bindingFor("cmd+\\")?.command).toBe("toggle-sidebar");
    expect(bindingFor("cmd+shift+e")?.command).toBe("export-pdf");
  });
});

describe("the menu bridge", () => {
  it("maps every menu id src-tauri/src/lib.rs emits to a real command", () => {
    const ids = new Set(COMMANDS.map((c) => c.id));
    for (const id of MENU_IDS) expect(ids.has(id)).toBe(true);
  });

  // macOS performs a key equivalent by firing the menu item itself, with no idea which context has
  // the keyboard, so a row for the link tool would open its popover over an open palette and the
  // document context this binding sits in would mean nothing. Adding the row later means changing
  // this line, which is the point of it being here.
  it("leaves the link off the native menu, since its chord is context dependent", () => {
    expect(MENU_IDS).not.toContain("insert-link");
  });
});
