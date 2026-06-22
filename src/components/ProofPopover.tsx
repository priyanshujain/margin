import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { severityFor, type ProofCoords, type ProofIssue } from "../editor/proofing";

function humanize(category: string): string {
  return category.replace(/([a-z])([A-Z])/g, "$1 $2");
}

interface ProofPopoverProps {
  issue: ProofIssue;
  coords: ProofCoords;
  onReplace: (suggestion: string) => void;
  onIgnore: () => void;
  onRemember: () => void;
  onClose: () => void;
}

const POP_WIDTH = 264;

export function ProofPopover({ issue, coords, onReplace, onIgnore, onRemember, onClose }: ProofPopoverProps) {
  const popRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (popRef.current?.contains(e.target as Node)) return;
      onClose();
    };
    const close = () => onClose();
    document.addEventListener("mousedown", onDown, true);
    document.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      document.removeEventListener("mousedown", onDown, true);
      document.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  }, [onClose]);

  const left = Math.min(Math.max(8, coords.left - POP_WIDTH / 2), window.innerWidth - POP_WIDTH - 8);
  const top = coords.bottom + 6;

  return createPortal(
    <div
      ref={popRef}
      className="proof-pop"
      style={{ left, top, width: POP_WIDTH }}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <div className={`proof-pop-kind sev-${severityFor(issue.category)}`}>{humanize(issue.category)}</div>
      <div className="proof-pop-msg">{issue.message}</div>
      {issue.suggestions.length > 0 && (
        <div className="proof-pop-suggestions">
          {issue.suggestions.map((suggestion, i) => (
            <button key={i} className="proof-suggestion" onClick={() => onReplace(suggestion)}>
              {suggestion === "" ? "Remove" : suggestion}
            </button>
          ))}
        </div>
      )}
      <div className="proof-pop-actions">
        <button className="proof-action" onClick={onIgnore}>
          Ignore
        </button>
        {issue.kind === "spelling" && (
          <button className="proof-action" onClick={onRemember}>
            Remember
          </button>
        )}
      </div>
    </div>,
    document.body
  );
}
