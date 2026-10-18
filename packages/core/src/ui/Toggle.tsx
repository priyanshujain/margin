import { useId } from "react";
import "./Toggle.css";

export interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  note?: string;
  disabled?: boolean;
}

export function Toggle({ checked, onChange, label, note, disabled }: ToggleProps) {
  const id = useId();

  const control = (
    <button
      type="button"
      id={id}
      role="switch"
      aria-checked={checked}
      aria-label={label ? undefined : "Toggle"}
      className="toggle"
      data-on={checked ? "" : undefined}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <span className="toggle-knob" />
    </button>
  );

  if (!label) return control;

  return (
    <div className="toggle-row">
      <span className="toggle-text">
        <label className="toggle-label" htmlFor={id}>
          {label}
        </label>
        {note ? <span className="toggle-note">{note}</span> : null}
      </span>
      {control}
    </div>
  );
}
