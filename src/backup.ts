import { invoke } from "@tauri-apps/api/core";
import { isDesktop } from "./ipc";

export interface BackupStatus {
  connected: boolean;
  email: string | null;
  lastBackup: number | null;
  pending: boolean;
}

export interface RestoreResult {
  restored: number;
}

export interface BackupOutcome extends BackupStatus {
  uploaded: number;
}

export interface RemoteBackup {
  name: string;
}

const OFFLINE: BackupStatus = { connected: false, email: null, lastBackup: null, pending: false };

export async function gdriveStatus(): Promise<BackupStatus> {
  if (!isDesktop) return OFFLINE;
  return invoke<BackupStatus>("gdrive_status");
}

export async function gdriveConnect(): Promise<string> {
  return invoke<string>("gdrive_connect");
}

export async function gdriveDisconnect(): Promise<BackupStatus> {
  return invoke<BackupStatus>("gdrive_disconnect");
}

export async function gdriveBackup(): Promise<BackupOutcome> {
  return invoke<BackupOutcome>("gdrive_backup");
}

export async function gdriveRestore(): Promise<RestoreResult> {
  return invoke<RestoreResult>("gdrive_restore");
}

export async function gdriveListBackups(): Promise<RemoteBackup[]> {
  if (!isDesktop) return [];
  return invoke<RemoteBackup[]>("gdrive_list_backups");
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
