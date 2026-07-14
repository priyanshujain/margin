import { useEffect, useRef, type ReactNode } from "react";
import { Icon } from "./Icon";

interface ConfirmDialogProps {
  title: string;
  message: ReactNode;
  confirmLabel?: string;
  onConfirm: () => void;
  onClose: () => void;
}

export function ConfirmDialog({ title, message, confirmLabel = "Delete", onConfirm, onClose }: ConfirmDialogProps) {
  const confirmRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    confirmRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="overlay" onClick={onClose}>
      <div className="panel panel-confirm" onClick={(e) => e.stopPropagation()}>
        <div className="panel-head">
          <h2>{title}</h2>
          <button className="icon-btn" onClick={onClose} title="Close">
            <Icon d="M6 6l12 12M18 6L6 18" />
          </button>
        </div>
        <div className="panel-body">
          <p className="confirm-text">{message}</p>
        </div>
        <div className="panel-foot">
          <button className="btn-ghost" onClick={onClose}>
            Cancel
          </button>
          <button ref={confirmRef} className="btn-danger" onClick={onConfirm}>
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
