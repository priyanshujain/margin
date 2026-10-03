import { useRef } from "react";
import { useEscapeLayer } from "../escape";
import { useFocusTrap } from "../focus";
import { isDesktop } from "../ipc";
import { Icon } from "./Icon";
import "../styles/shortcuts.css";

const mac = navigator.platform.includes("Mac");
const mod = mac ? "⌘" : "Ctrl";
const alt = mac ? "⌥" : "Alt";
const shift = mac ? "⇧" : "Shift";

const groups: { title: string; shortcuts: [string, string[]][] }[] = [
  {
    title: "Projects",
    shortcuts: [
      ["New project", [mod, "N"]],
      ["Save", [mod, "S"]],
      ["Settings", [mod, ","]],
      ["Export PDF", [mod, shift, "P"]],
      ["Export EPUB", [mod, shift, "E"]],
      ["Keyboard shortcuts", [mod, shift, "/"]],
      ...(mac && isDesktop ? [["Open window", [mod, shift, "M"]] as [string, string[]]] : []),
    ],
  },
  {
    title: "Editing",
    shortcuts: [
      ["Undo", [mod, "Z"]],
      ["Redo", [mod, shift, "Z"]],
      ["Cut", [mod, "X"]],
      ["Copy", [mod, "C"]],
      ["Paste", [mod, "V"]],
      ["Paste without formatting", [mod, shift, "V"]],
      ["Select all", [mod, "A"]],
    ],
  },
  {
    title: "Formatting",
    shortcuts: [
      ["Bold", [mod, "B"]],
      ["Italic", [mod, "I"]],
      ["Underline", [mod, "U"]],
      ["Strikethrough", [mod, shift, "X"]],
      ["Add or edit link", [mod, "K"]],
      ["Heading", [mod, alt, "2 / 3"]],
      ["Paragraph", [mod, alt, "0"]],
      ["Bullet list", [mod, shift, "8"]],
      ["Numbered list", [mod, shift, "7"]],
      ["Task list", [mod, shift, "9"]],
      ["Quote", [mod, shift, "B"]],
      ["Align left", [mod, shift, "L"]],
      ...(!isDesktop ? [["Align center", [mod, shift, "E"]] as [string, string[]]] : []),
      ["Align right", [mod, shift, "R"]],
      ["Indent / remove indent", ["Tab / Shift Tab"]],
      ["Line break", [shift, "Enter"]],
    ],
  },
  {
    title: "Navigation and search",
    shortcuts: [
      ["Find", [mod, "F"]],
      ...(mac ? [
        ["Find and replace", [mod, alt, "F"]],
        ["Toggle chapters", [mod, "\\"]],
        ["Next / previous chapter", [mod, alt, "↓ / ↑"]],
        ["Next / previous chapter", [mod, alt, "→ / ←"]],
      ] as [string, string[]][] : []),
      ["Next chapter", ["Ctrl", "Tab"]],
      ["Previous chapter", ["Ctrl", shift, "Tab"]],
      ["Next / previous search result", ["Enter / Shift Enter"]],
      ["Close popup", ["Esc"]],
      ["Return to all projects", ["Esc twice"]],
    ],
  },
  ...(mac && isDesktop ? [{
    title: "Apple Writing Tools",
    shortcuts: [
      ["Proofread", [alt, shift, "F"]],
      ["Rewrite", [alt, shift, "R"]],
    ] as [string, string[]][],
  }] : []),
];

export function KeyboardShortcuts({ open, onClose }: { open: boolean; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useEscapeLayer(open, onClose);
  useFocusTrap(ref, open);
  if (!open) return null;

  return (
    <div className="overlay" onClick={onClose}>
      <div ref={ref} className="panel shortcuts-panel" role="dialog" aria-modal="true" aria-labelledby="shortcuts-title" onClick={(event) => event.stopPropagation()}>
        <div className="panel-head">
          <h2 id="shortcuts-title">Keyboard shortcuts</h2>
          <button className="icon-btn" onClick={onClose} title="Close" aria-label="Close keyboard shortcuts">
            <Icon d="M6 6l12 12M18 6L6 18" />
          </button>
        </div>
        <div className="panel-body shortcuts-groups">
          {groups.map((group) => (
            <section key={group.title} className="shortcuts-group">
              <h3>{group.title}</h3>
              <dl>
                {group.shortcuts.map(([label, keys]) => (
                  <div className="shortcut-row" key={label + keys.join()}>
                    <dt>{label}</dt>
                    <dd>{keys.map((key) => <kbd key={key}>{key}</kbd>)}</dd>
                  </div>
                ))}
              </dl>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
