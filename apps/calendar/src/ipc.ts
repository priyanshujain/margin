// The IPC contract. Every type here mirrors a struct in src-tauri/src/dto.rs. Both sides are
// frozen once written: implementation modules add bodies, not fields.
//
// Typed per-command wrappers live in src/api/, grouped by domain.

import { createCall } from "@margin/core/ipc";

export { isDesktop, isMacDesktop, isTauri, live } from "@margin/core/ipc";

export interface Account {
  id: string;
  email: string;
  connected: boolean;
  /** google, and the providers that follow it. */
  provider: string;
  /** Where a credential account's calendars live. Null for Google. */
  server: string | null;
}

/** The account id every calendar that lives only on this device is filed under. */
export const LOCAL_ACCOUNT = "local";

/** The account id every subscribed iCalendar feed is filed under. */
export const SUBSCRIBED_ACCOUNT = "ics";

export interface Calendar {
  id: string;
  accountId: string;
  summary: string;
  description: string | null;
  colorHex: string;
  selected: boolean;
  accessRole: string;
  timeZone: string;
  primary: boolean;
}

export interface Attendee {
  email: string;
  displayName: string | null;
  responseStatus: "needsAction" | "declined" | "tentative" | "accepted";
  organizer: boolean;
  self: boolean;
  optional: boolean;
}

export interface Conference {
  kind: string;
  uri: string | null;
  label: string | null;
}

/** Identifies one occurrence of a series, stable across syncs. */
export interface InstanceKey {
  eventId: string;
  originalStart: string | null;
}

/**
 * One event occurrence, already expanded and already converted to the local zone.
 *
 * `start` and `end` are RFC3339 with an offset, except when `allDay`, where they are date-only
 * `YYYY-MM-DD` and must never be shifted into a zone. `startMs` and `endMs` are epoch
 * milliseconds, with all-day events pinned to local midnight, and `endMs` is exclusive.
 */
export interface Instance {
  eventId: string;
  calendarId: string;
  accountId: string;
  originalStart: string | null;
  start: string;
  end: string;
  startMs: number;
  endMs: number;
  allDay: boolean;
  summary: string;
  description: string | null;
  location: string | null;
  status: "confirmed" | "tentative" | "cancelled";
  recurring: boolean;
  /** Resolved for rendering: the event's own colour when it has one, else its calendar's. */
  colorHex: string;
  /** Google's per-event colour id, "1" to "11". Null means the event follows its calendar. */
  colorId: string | null;
  etag: string | null;
  organizer: string | null;
  attendees: Attendee[];
  conference: Conference | null;
  readOnly: boolean;
  /** Written locally and still sitting in the outbox. */
  pending: boolean;
}

export interface EventDraft {
  calendarId: string;
  summary: string;
  description?: string | null;
  location?: string | null;
  start: string;
  end: string;
  allDay: boolean;
  /** Raw RFC5545 lines (RRULE/RDATE/EXDATE), omitted for a one-off. */
  recurrence?: string[];
  /** Google's per-event colour id, "1" to "11". Omit to follow the calendar. */
  colorId?: string;
}

/** An absent field means unchanged. An empty string on `description` or `location` clears it. */
export interface EventPatch {
  summary?: string;
  description?: string;
  location?: string;
  start?: string;
  end?: string;
  allDay?: boolean;
  calendarId?: string;
  recurrence?: string[];
  /** An empty string clears it back to the calendar's colour. */
  colorId?: string;
}

export type Scope = "this" | "following" | "all";

/**
 * Google's per-event palette, keyed by `colorId`. These are Google's own hex values, so a colour
 * chosen here reads the same in both apps. Rendering maps them onto this app's own hues.
 */
export const EVENT_COLORS: { id: string; hex: string; name: string }[] = [
  { id: "1", hex: "#7986cb", name: "Lavender" },
  { id: "2", hex: "#33b679", name: "Sage" },
  { id: "3", hex: "#8e24aa", name: "Grape" },
  { id: "4", hex: "#e67c73", name: "Flamingo" },
  { id: "5", hex: "#f6bf26", name: "Banana" },
  { id: "6", hex: "#f4511e", name: "Tangerine" },
  { id: "7", hex: "#039be5", name: "Peacock" },
  { id: "8", hex: "#616161", name: "Graphite" },
  { id: "9", hex: "#3f51b5", name: "Blueberry" },
  { id: "10", hex: "#0b8043", name: "Basil" },
  { id: "11", hex: "#d50000", name: "Tomato" },
];

export interface SyncStatus {
  phase: "idle" | "syncing" | "error";
  /** Epoch milliseconds of the last successful sync. */
  lastSync: number | null;
  error: string | null;
  pendingWrites: number;
  message: string | null;
}

/** Payload of the `auth` event. */
export interface AuthEvent {
  ok: boolean;
  error: string | null;
  accountId: string | null;
  email: string | null;
  /** The consent browser was closed by hand. Not `ok`, but not a failure to report either. */
  cancelled: boolean;
}

/** Payload of `sync-progress`. `store-changed` carries a plain reason string. */
export type SyncProgress = SyncStatus;

/**
 * In Tauri this is `invoke`, with a failure written to the log before it becomes a toast. Opened in
 * a browser during development it is served from the dev fixture in `src/dev/mockIpc.ts`.
 */
export const call = createCall({
  mock: import.meta.env.DEV ? () => import("./dev/mockIpc").then((m) => m.mockCall) : undefined,
});
