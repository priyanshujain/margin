import { Icon } from "./Icon";
import { useBackup } from "../store/useBackup";
import { formatBackupTime } from "../backup";

export function BackupButton() {
  const connected = useBackup((s) => s.connected);
  const pending = useBackup((s) => s.pending);
  const phase = useBackup((s) => s.phase);
  const lastBackup = useBackup((s) => s.lastBackup);
  const openSettings = useBackup((s) => s.openSettings);

  const busy = phase === "working" || phase === "connecting";
  const error = phase === "error";

  const state = busy ? "working" : error ? "error" : !connected ? "off" : pending ? "pending" : "synced";

  const title =
    phase === "connecting"
      ? "Waiting for Google authorization…"
      : state === "working"
        ? "Backing up…"
        : state === "error"
          ? "Backup needs attention"
          : state === "off"
            ? "Connect Google Drive"
            : state === "pending"
              ? "Changes not backed up"
              : formatBackupTime(lastBackup);

  const onClick = () => {
    openSettings();
  };

  return (
    <button className="icon-btn backup-btn" data-state={state} onClick={onClick} title={title}>
      {busy ? (
        <Icon size={18}>
          <path d="M21 12a9 9 0 1 1-2.6-6.4M21 4v5h-5" />
        </Icon>
      ) : (
        <Icon size={18}>
          <path d="M7 18a4 4 0 0 1 0-8 5 5 0 0 1 9.6-1.3A3.5 3.5 0 0 1 18 18H7z" />
          {connected ? <path d="M12 16.5v-5M9.7 12.8l2.3-2.3 2.3 2.3" /> : null}
        </Icon>
      )}
    </button>
  );
}
