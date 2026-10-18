import type { ReactNode } from "react";
import "./Icon.css";

export interface IconProps {
  d?: string;
  size?: number;
  children?: ReactNode;
}

export function Icon({ d, size = 16, children }: IconProps) {
  return (
    <svg
      className="icon"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children ?? <path d={d} />}
    </svg>
  );
}
