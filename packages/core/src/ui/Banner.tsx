import type { ReactNode } from "react";
import { Icon } from "./Icon";
import "./Banner.css";

export interface BannerProps {
  children: ReactNode;
  icon?: string;
  tone?: "default" | "muted";
  action?: {
    label: string;
    onClick: () => void;
    busy?: boolean;
    busyLabel?: string;
  };
}

export function Banner({ children, icon, tone = "default", action }: BannerProps) {
  return (
    <div className="banner" data-tone={tone}>
      {icon ? <Icon d={icon} size={14} /> : null}
      <span className="banner-text">{children}</span>
      {action ? (
        <button
          type="button"
          className="banner-action"
          data-busy={action.busy ? "" : undefined}
          aria-busy={action.busy ? "true" : undefined}
          disabled={action.busy}
          onClick={action.onClick}
        >
          {action.busy ? (action.busyLabel ?? `${action.label}…`) : action.label}
        </button>
      ) : null}
    </div>
  );
}
