import { call, live } from "./ipc";

export interface BackupStatus {
  connected: boolean;
  email: string | null;
  lastBackup: number | null;
  pending: boolean;
  needsReauth: boolean;
}

export interface RestoreResult {
  restored: number;
}

export interface BackupOutcome extends BackupStatus {
  uploaded: number;
}

export interface SyncOutcome extends BackupStatus {
  uploaded: number;
  downloaded: number;
}

export interface RemoteBackup {
  name: string;
}

const OFFLINE: BackupStatus = { connected: false, email: null, lastBackup: null, pending: false, needsReauth: false };

export async function gdriveStatus(): Promise<BackupStatus> {
  if (!live()) return OFFLINE;
  return call<BackupStatus>("gdrive_status");
}

export async function gdriveConnect(): Promise<string> {
  return call<string>("gdrive_connect");
}

export async function gdriveDisconnect(): Promise<BackupStatus> {
  return call<BackupStatus>("gdrive_disconnect");
}

export async function gdriveBackup(): Promise<BackupOutcome> {
  return call<BackupOutcome>("gdrive_backup");
}

export async function gdriveSync(): Promise<SyncOutcome> {
  return call<SyncOutcome>("gdrive_sync");
}

export async function gdriveRestore(): Promise<RestoreResult> {
  return call<RestoreResult>("gdrive_restore");
}

export async function gdriveListBackups(): Promise<RemoteBackup[]> {
  if (!live()) return [];
  return call<RemoteBackup[]>("gdrive_list_backups");
}

export function formatBackupTime(ts: number | null): string {
  if (!ts) return "Not backed up yet";
  const diff = Date.now() - ts * 1000;
  const min = Math.floor(diff / 60000);
  if (min < 1) return "Backed up just now";
  if (min < 60) return `Backed up ${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `Backed up ${hr}h ago`;
  const day = Math.floor(hr / 24);
  if (day === 1) return "Backed up yesterday";
  if (day < 7) return `Backed up ${day}d ago`;
  return `Backed up ${new Date(ts * 1000).toLocaleDateString()}`;
}
