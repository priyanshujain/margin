import { useRef } from "react";
import { useUpdater } from "../store/useUpdater";
import { dismissUpdate } from "../updater";
import { useEscapeLayer } from "../escape";
import { useFocusTrap } from "../focus";
import { Icon } from "./Icon";

const TITLES: Record<string, string> = {
  checking: "Check for Updates",
  downloading: "Updating margin",
  installing: "Updating margin",
  uptodate: "Check for Updates",
  error: "Update Failed",
};

export function UpdateDialog() {
  const phase = useUpdater((s) => s.phase);
  const downloaded = useUpdater((s) => s.downloaded);
  const total = useUpdater((s) => s.total);
  const error = useUpdater((s) => s.error);

  const busy = phase === "downloading" || phase === "installing";
  const panelRef = useRef<HTMLDivElement>(null);
  useEscapeLayer(phase !== "idle" && !busy, dismissUpdate);
  useFocusTrap(panelRef, phase !== "idle");

  if (phase === "idle") return null;

  const pct = total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
  const close = busy ? undefined : dismissUpdate;

  return (
    <div className="overlay" onClick={close}>
      <div ref={panelRef} className="panel panel-confirm" onClick={(e) => e.stopPropagation()}>
        <div className="panel-head">
          <h2>{TITLES[phase]}</h2>
          {!busy && (
            <button className="icon-btn" onClick={dismissUpdate} title="Close">
              <Icon d="M6 6l12 12M18 6L6 18" />
            </button>
          )}
        </div>

        <div className="panel-body">
          {phase === "checking" && (
            <div className="update-status-row">
              <span className="update-spinner" />
              <span>Checking for updates…</span>
            </div>
          )}

          {phase === "uptodate" && <p className="confirm-text">margin is up to date.</p>}

          {phase === "downloading" && (
            <>
              <div className={`update-progress${total > 0 ? "" : " indeterminate"}`}>
                <div className="update-progress-fill" style={total > 0 ? { width: `${pct}%` } : undefined} />
              </div>
              <p className="update-status">{total > 0 ? `Downloading update… ${pct}%` : "Downloading update…"}</p>
            </>
          )}

          {phase === "installing" && (
            <div className="update-status-row">
              <span className="update-spinner" />
              <span>Installing update. margin will restart…</span>
            </div>
          )}

          {phase === "error" && (
            <>
              <p className="confirm-text">Could not update margin.</p>
              {error && <div className="update-notes">{error}</div>}
            </>
          )}
        </div>

        {(phase === "uptodate" || phase === "error") && (
          <div className="panel-foot">
            <button className="btn-primary" onClick={dismissUpdate}>
              OK
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
