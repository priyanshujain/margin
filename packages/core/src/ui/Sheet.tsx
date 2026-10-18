import { useEffect, useRef, type ReactNode } from "react";
import { useEscapeLayer } from "../hooks/escape";
import { Button } from "./Button";
import { CHEVRON_LEFT, CLOSE } from "../icons";
import "./Sheet.css";

export interface SheetProps {
  open: boolean;
  title: string;
  onClose: () => void;
  size?: "mini" | "default" | "wide" | "full";
  foot?: ReactNode;
  onBack?: () => void;
  backLabel?: string;
  busy?: boolean;
  children: ReactNode;
}

export function Sheet({
  open,
  title,
  onClose,
  size = "default",
  foot,
  onBack,
  backLabel = "the last panel",
  busy,
  children,
}: SheetProps) {
  useEscapeLayer(open, busy ? () => {} : (onBack ?? onClose));
  const panel = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    const el = panel.current;
    if (!el) return;
    (el.querySelector<HTMLElement>("[data-autofocus]") ?? el).focus();
  }, [open]);

  if (!open) return null;

  return (
    <div
      className="overlay"
      onPointerDown={(e) => {
        if (e.target === e.currentTarget && !busy) onClose();
      }}
    >
      <div
        className="panel sheet"
        data-size={size}
        data-busy={busy ? "" : undefined}
        aria-busy={busy || undefined}
        ref={panel}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="panel-head">
          {onBack ? (
            <Button
              variant="ghost"
              iconOnly
              icon={CHEVRON_LEFT}
              title={`Back to ${backLabel} (⎋)`}
              label={`Back to ${backLabel}`}
              disabled={busy}
              onClick={onBack}
            />
          ) : null}
          <h2>{title}</h2>
          <Button
            variant="ghost"
            iconOnly
            icon={CLOSE}
            title="Close (⎋)"
            label="Close"
            disabled={busy}
            onClick={onClose}
          />
        </div>
        <div className="panel-body">{children}</div>
        {foot ? <div className="panel-foot">{foot}</div> : null}
      </div>
    </div>
  );
}

export interface ConfirmProps {
  title: string;
  body: ReactNode;
  confirmLabel: string;
  busy?: boolean;
  busyLabel?: string;
  variant?: "danger" | "primary";
  onConfirm: () => void;
  onCancel: () => void;
}

export function Confirm({
  title,
  body,
  confirmLabel,
  busy,
  busyLabel,
  variant = "danger",
  onConfirm,
  onCancel,
}: ConfirmProps) {
  useEscapeLayer(true, busy ? () => {} : onCancel);
  const cancel = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    cancel.current?.querySelector("button")?.focus();
  }, []);

  return (
    <div className="confirm" data-busy={busy ? "" : undefined}>
      <h3 className="confirm-title">{title}</h3>
      <div className="confirm-body">{body}</div>
      <div className="confirm-actions" ref={cancel}>
        <Button disabled={busy} onClick={onCancel}>
          Cancel
        </Button>
        <Button variant={variant} disabled={busy} onClick={onConfirm}>
          {busy ? (busyLabel ?? `${confirmLabel}…`) : confirmLabel}
        </Button>
      </div>
    </div>
  );
}
