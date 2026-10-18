import { useEffect } from "react";
import { runCommand } from "./commands";
import {
  allBindings,
  bindingsVersion,
  normalizeCombo,
  primaryHeld,
  secondaryHeld,
  type Binding,
  type KeyContext,
} from "./bindings";

let index = new Map<string, Binding[]>();
let indexed = -1;

function lookup(combo: string): Binding[] | undefined {
  if (indexed !== bindingsVersion()) {
    index = new Map();
    for (const binding of allBindings()) {
      for (const key of binding.keys) {
        const normal = normalizeCombo(key);
        const found = index.get(normal);
        if (found) found.push(binding);
        else index.set(normal, [binding]);
      }
    }
    indexed = bindingsVersion();
  }
  return index.get(combo);
}

interface Frame {
  context: KeyContext;
}

const stack: Frame[] = [];

const activeContext = (): KeyContext => stack[stack.length - 1]?.context ?? "view";

export function pushContext(context: KeyContext): () => void {
  const frame: Frame = { context };
  stack.push(frame);
  return () => {
    const at = stack.indexOf(frame);
    if (at !== -1) stack.splice(at, 1);
  };
}

export function useKeyContext(context: KeyContext, active = true): void {
  useEffect(() => {
    if (!active) return;
    return pushContext(context);
  }, [context, active]);
}

export function comboOf(e: KeyboardEvent): string {
  const mods =
    (primaryHeld(e) ? "cmd+" : "") +
    (secondaryHeld(e) ? "ctrl+" : "") +
    (e.altKey ? "alt+" : "") +
    (e.shiftKey ? "shift+" : "");
  const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
  return `${mods}${key}`;
}

const SHADOWS_VIEW: readonly KeyContext[] = ["overlay", "editor"];

function resolve(combo: string): Binding | null {
  const candidates = lookup(combo);
  if (!candidates) return null;
  const top = activeContext();

  const inTop = candidates.find((b) => b.context === top);
  if (inTop) return inTop;

  if (!SHADOWS_VIEW.includes(top)) {
    const inView = candidates.find((b) => b.context === "view");
    if (inView) return inView;
  }
  return candidates.find((b) => b.context === "global") ?? null;
}

function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el || typeof el.tagName !== "string") return false;
  if (el.isContentEditable) return true;
  return el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT";
}

function isActivatable(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el || typeof el.tagName !== "string" || el.tabIndex < 0) return false;
  return (
    el.tagName === "BUTTON" ||
    el.tagName === "A" ||
    el.tagName === "SUMMARY" ||
    el.getAttribute("role") === "button"
  );
}

interface Family {
  combos: readonly string[];
  run: (index: number) => void;
}

const families: Family[] = [];

export function registerKeyFamily(combos: readonly string[], run: (index: number) => void): () => void {
  const family: Family = { combos: combos.map(normalizeCombo), run };
  families.push(family);
  return () => {
    const at = families.indexOf(family);
    if (at !== -1) families.splice(at, 1);
  };
}

function onKeyDown(e: KeyboardEvent): void {
  if (e.isComposing || e.defaultPrevented) return;
  if (e.key === "Escape") return;
  if ((e.key === "Enter" || e.key === " ") && isActivatable(e.target)) return;

  const combo = comboOf(e);

  if (activeContext() !== "overlay") {
    for (let i = families.length - 1; i >= 0; i--) {
      const at = families[i].combos.indexOf(combo);
      if (at === -1) continue;
      e.preventDefault();
      families[i].run(at);
      return;
    }
  }

  const binding = resolve(combo);
  if (!binding) return;

  if (!binding.allowInInput && (isTyping(e.target) || isTyping(document.activeElement))) return;

  if (binding.command === null) return;

  e.preventDefault();
  e.stopPropagation();
  runCommand(binding.command);
}

let installs = 0;

export function installKeymap(): () => void {
  installs += 1;
  if (installs === 1) window.addEventListener("keydown", onKeyDown, true);
  return () => {
    installs -= 1;
    if (installs === 0) window.removeEventListener("keydown", onKeyDown, true);
  };
}

export function useKeymap(): void {
  useEffect(() => installKeymap(), []);
}
