import { useEffect, useState } from "react";
import { Icon } from "./Icon";
import { ConfirmDialog } from "./ConfirmDialog";
import { useBackup } from "../store/useBackup";
import { formatBackupTime, gdriveListBackups } from "../backup";

export function BackupSettings() {
  const open = useBackup((s) => s.settingsOpen);
  const close = useBackup((s) => s.closeSettings);
  const connected = useBackup((s) => s.connected);
  const email = useBackup((s) => s.email);
  const lastBackup = useBackup((s) => s.lastBackup);
  const pending = useBackup((s) => s.pending);
  const phase = useBackup((s) => s.phase);
  const connect = useBackup((s) => s.connect);
  const disconnect = useBackup((s) => s.disconnect);
  const backup = useBackup((s) => s.backup);
  const restore = useBackup((s) => s.restore);
  const cancelConnect = useBackup((s) => s.cancelConnect);
  const openAuthUrl = useBackup((s) => s.openAuthUrl);
  const copyAuthUrl = useBackup((s) => s.copyAuthUrl);

  const [remoteCount, setRemoteCount] = useState<number | null>(null);
  const [confirmRestore, setConfirmRestore] = useState(false);
  const working = phase === "working";

  useEffect(() => {
    if (!open || !connected) {
      setRemoteCount(null);
      return;
    }
    let active = true;
    gdriveListBackups()
      .then((list) => active && setRemoteCount(list.length))
      .catch(() => active && setRemoteCount(null));
    return () => {
      active = false;
    };
  }, [open, connected, lastBackup]);

  if (!open) return null;

  return (
    <>
      <div className="overlay" onClick={close}>
        <div className="panel" onClick={(e) => e.stopPropagation()}>
          <div className="panel-head">
            <h2>Backup &amp; Sync</h2>
            <button className="icon-btn" onClick={close} title="Close">
              <Icon d="M6 6l12 12M18 6L6 18" />
            </button>
          </div>
          <div className="panel-body">
            {phase === "connecting" ? (
              <div className="backup-connecting">
                <div className="backup-waiting">
                  <Icon size={16}>
                    <path d="M21 12a9 9 0 1 1-2.6-6.4M21 4v5h-5" />
                  </Icon>
                  <span>Waiting for Google authorization…</span>
                </div>
                <p>Finish in the browser tab that opened. If it didn't open, reopen the link or copy it into a browser.</p>
                <div className="backup-link-row">
                  <button className="backup-text-btn" onClick={cancelConnect}>
                    Stop linking
                  </button>
                  <div className="backup-links">
                    <button className="backup-text-btn" onClick={openAuthUrl}>
                      Open link again
                    </button>
                    <span className="backup-link-sep">·</span>
                    <button className="backup-text-btn" onClick={copyAuthUrl}>
                      Copy link
                    </button>
                  </div>
                </div>
              </div>
            ) : !connected ? (
              <div className="backup-intro">
                <p>
                  Connect Google Drive to keep a private backup of every book. margin only ever sees the files it
                  creates in a <strong>margin</strong> folder, never the rest of your Drive.
                </p>
                <button className="btn-primary" disabled={working} onClick={connect}>
                  {working ? "Connecting…" : "Connect Google Drive"}
                </button>
              </div>
            ) : (
              <>
                <div className="backup-account">
                  <div className="backup-account-info">
                    <span className="field-label">Connected account</span>
                    <span className="backup-email">{email || "Google Drive"}</span>
                  </div>
                  <button className="btn-ghost" disabled={working} onClick={disconnect}>
                    Disconnect
                  </button>
                </div>

                <div className="backup-status-row">
                  <span className={`backup-dot ${pending ? "is-pending" : "is-synced"}`} />
                  <span>{pending ? "Changes not backed up" : formatBackupTime(lastBackup)}</span>
                </div>

                <div className="backup-actions">
                  <button className="btn-primary" disabled={working} onClick={() => backup()}>
                    {working ? "Working…" : "Back up now"}
                  </button>
                  <button
                    className="btn-ghost"
                    disabled={working || remoteCount === 0}
                    onClick={() => setConfirmRestore(true)}
                  >
                    {remoteCount === null ? "Restore from Drive" : `Restore ${remoteCount} book${remoteCount === 1 ? "" : "s"}`}
                  </button>
                </div>

                <p className="backup-note">
                  Backs up automatically when you close margin, and every 15 minutes while there are changes.
                </p>
              </>
            )}
          </div>
        </div>
      </div>

      {confirmRestore && (
        <ConfirmDialog
          title="Restore from Google Drive"
          message={
            <>
              This replaces your local books with the copies in Google Drive. Any local changes that haven't been backed
              up will be lost.
            </>
          }
          confirmLabel="Restore"
          onConfirm={() => {
            setConfirmRestore(false);
            restore();
          }}
          onClose={() => setConfirmRestore(false)}
        />
      )}
    </>
  );
}
