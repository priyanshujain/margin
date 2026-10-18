import { useId } from "react";
import { NO_AUTOFILL } from "./autofill";
import "./Field.css";

export interface FieldProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  multiline?: boolean;
  rows?: number;
  hint?: string;
  tone?: "default" | "error";
  type?: "text" | "email" | "search" | "password";
  autoComplete?: "email" | "name" | "username" | "current-password" | "new-password";
  disabled?: boolean;
  autoFocus?: boolean;
}

export function Field({
  label,
  value,
  onChange,
  placeholder,
  multiline,
  rows = 4,
  hint,
  tone = "default",
  type = "text",
  autoComplete,
  disabled,
  autoFocus,
}: FieldProps) {
  const id = useId();
  return (
    <div className="field">
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      {multiline ? (
        <textarea
          id={id}
          className="field-textarea"
          rows={rows}
          value={value}
          placeholder={placeholder}
          disabled={disabled}
          autoFocus={autoFocus}
          data-autofocus={autoFocus ? "" : undefined}
          autoComplete="off"
          onChange={(e) => onChange(e.target.value)}
        />
      ) : (
        <input
          id={id}
          className="field-input"
          type={type}
          value={value}
          placeholder={placeholder}
          disabled={disabled}
          autoFocus={autoFocus}
          data-autofocus={autoFocus ? "" : undefined}
          {...NO_AUTOFILL}
          autoComplete={autoComplete ?? NO_AUTOFILL.autoComplete}
          onChange={(e) => onChange(e.target.value)}
        />
      )}
      {hint ? (
        <span className="field-hint" data-tone={tone}>
          {hint}
        </span>
      ) : null}
    </div>
  );
}
