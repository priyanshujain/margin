import type { ReactNode } from "react";
import { Icon } from "./Icon";

interface ConfirmDialogProps {
  title: string;
  message: ReactNode;
  confirmLabel?: string;
  onConfirm: () => void;
  onClose: () => void;
}

export function ConfirmDialog({ title, message, confirmLabel = "Delete", onConfirm, onClose }: ConfirmDialogProps) {
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
          <button className="btn-danger" onClick={onConfirm}>
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
