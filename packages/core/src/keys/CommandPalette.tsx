import { useEffect, useMemo, useState } from "react";
import { Palette, type PaletteGroup, type PaletteItem } from "../ui/Palette";
import { keyLabel, paletteCommands, type CommandBinding } from "./bindings";
import { commandMatches, runCommand } from "./commands";

export function bindingCaps(binding: CommandBinding): string[] {
  const combo = binding.keys[0];
  if (!combo) return [];
  return [combo.includes("+") ? keyLabel(combo) : combo];
}

export function commandItems(bindings: readonly CommandBinding[], query: string): PaletteItem[] {
  return bindings
    .filter((b) => commandMatches(b.label, query))
    .map((b) => ({ id: b.command, label: b.label, keys: bindingCaps(b) }));
}

export function defaultSections(query: string): PaletteGroup[] {
  return [{ id: "actions", label: "Actions", items: commandItems(paletteCommands(), query) }];
}

export interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  sections?: (query: string) => PaletteGroup[];
  onChoose?: (id: string) => boolean;
  onOpen?: () => void;
  placeholder?: string;
  empty?: string;
}

export function CommandPalette({
  open,
  onClose,
  sections = defaultSections,
  onChoose,
  onOpen,
  placeholder,
  empty,
}: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActive(null);
      onOpen?.();
    }
  }, [open, onOpen]);

  const groups = useMemo(() => (open ? sections(query) : []), [open, sections, query]);
  const flat = useMemo(() => groups.flatMap((g) => g.items.map((i) => i.id)), [groups]);
  const activeId = active && flat.includes(active) ? active : flat[0];

  const choose = (id: string) => {
    onClose();
    if (onChoose?.(id)) return;
    runCommand(id);
  };

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const at = flat.indexOf(activeId ?? "");
        const next = at + (e.key === "ArrowDown" ? 1 : -1);
        if (next >= 0 && next < flat.length) setActive(flat[next]);
      } else if (e.key === "Enter" && activeId) {
        e.preventDefault();
        choose(activeId);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <Palette
      open={open}
      query={query}
      onQuery={(next) => {
        setQuery(next);
        setActive(null);
      }}
      groups={groups}
      activeId={activeId}
      onChoose={choose}
      onClose={onClose}
      placeholder={placeholder}
      empty={empty}
    />
  );
}
