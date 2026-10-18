import type { ReactNode } from "react";
import "./EmptyState.css";

export interface EmptyStateProps {
  children: ReactNode;
}

export function EmptyState({ children }: EmptyStateProps) {
  return <p className="empty-state">{children}</p>;
}
