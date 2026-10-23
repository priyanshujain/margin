import { call, type StateSyncStatus, type SyncStorage } from "../ipc";

export const stateSyncStatus = () => call<StateSyncStatus>("state_sync_status");

/** `null` turns sync off on this machine. A blank secret keeps the one already stored. */
export const stateSyncConfigure = (storage: SyncStorage | null) =>
  call<StateSyncStatus>("state_sync_configure", { storage });

export const stateSyncNow = () => call<StateSyncStatus>("state_sync_now");

export const stateSyncPoll = () => call<StateSyncStatus>("state_sync_poll");

export const stateSyncPhraseCreate = () => call<string>("state_sync_phrase_create");

export const stateSyncPhraseShow = () => call<string>("state_sync_phrase_show");

export const stateSyncPhraseEnter = (phrase: string) =>
  call<StateSyncStatus>("state_sync_phrase_enter", { phrase });

/** The native folder picker; `null` when cancelled. The path comes back already checked. */
export const stateSyncPickFolder = () => call<string | null>("state_sync_pick_folder");
