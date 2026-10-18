import type { ReactNode } from "react";
import "./GroupHead.css";

export interface GroupHeadProps {
  children: ReactNode;
  action?: { label: string; onClick: () => void };
  rule?: boolean;
}

export function GroupHead({ children, action, rule }: GroupHeadProps) {
  return (
    <div className="group-head" data-rule={rule ? "" : undefined}>
      <span className="group-head-label">{children}</span>
      {rule ? <span className="group-head-rule" /> : null}
      {action ? (
        <button type="button" className="group-head-action" onClick={action.onClick}>
          {action.label}
        </button>
      ) : null}
    </div>
  );
}
