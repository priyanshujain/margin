import type { ReactNode } from "react";
import { Key } from "./Key";
import "./Toast.css";

export interface ToastProps {
  children: ReactNode;
  action?: { label: string; keycap?: string; onClick: () => void };
}

export function Toast({ children, action }: ToastProps) {
  return (
    <div className="toast" role="status">
      <span className="toast-text">{children}</span>
      {action ? (
        <button type="button" className="toast-action" onClick={action.onClick}>
          {action.label}
          {action.keycap ? <Key size="sm">{action.keycap}</Key> : null}
        </button>
      ) : null}
    </div>
  );
}
