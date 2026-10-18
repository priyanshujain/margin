import type { ReactNode, Ref } from "react";
import { Icon } from "./Icon";
import { Key } from "./Key";
import "./Button.css";

export type ButtonVariant = "default" | "primary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps {
  children?: ReactNode;
  onClick?: () => void;
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: string;
  keycap?: string;
  iconOnly?: boolean;
  active?: boolean;
  disabled?: boolean;
  title?: string;
  label?: string;
  type?: "button" | "submit";
  ref?: Ref<HTMLButtonElement>;
}

export function Button({
  children,
  onClick,
  variant = "default",
  size = "md",
  icon,
  keycap,
  iconOnly,
  active,
  disabled,
  title,
  label,
  type = "button",
  ref,
}: ButtonProps) {
  return (
    <button
      ref={ref}
      type={type}
      className="button"
      data-variant={variant}
      data-size={size}
      data-icon-only={iconOnly ? "" : undefined}
      data-active={active ? "" : undefined}
      disabled={disabled}
      title={title}
      aria-label={label ?? (iconOnly ? title : undefined)}
      onClick={onClick}
    >
      {icon ? <Icon d={icon} size={iconOnly ? 16 : 14} /> : null}
      {iconOnly ? null : children}
      {keycap && !iconOnly ? <Key size="sm">{keycap}</Key> : null}
    </button>
  );
}
