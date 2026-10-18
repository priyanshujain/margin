import { useEffect } from "react";
import { useToast } from "../hooks/useToast";
import { Toast } from "./Toast";

export interface ToastsProps {
  dismissMs?: number;
}

export function Toasts({ dismissMs = 6000 }: ToastsProps) {
  const message = useToast((s) => s.message);
  const action = useToast((s) => s.action);
  const seq = useToast((s) => s.seq);
  const dismiss = useToast((s) => s.dismiss);

  useEffect(() => {
    if (!message) return;
    const timer = window.setTimeout(dismiss, dismissMs);
    return () => window.clearTimeout(timer);
  }, [message, seq, dismiss, dismissMs]);

  if (!message) return null;

  return (
    <Toast
      action={
        action
          ? {
              label: action.label,
              keycap: action.keycap,
              onClick: () => {
                action.run();
                dismiss();
              },
            }
          : undefined
      }
    >
      {message}
    </Toast>
  );
}

