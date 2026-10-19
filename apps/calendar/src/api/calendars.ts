import { call, type Calendar } from "../ipc";

export const calendarsList = () => call<Calendar[]>("calendars_list");

export const calendarSetSelected = (calendarId: string, selected: boolean) =>
  call<void>("calendar_set_selected", { calendarId, selected });

export const calendarCreate = (name: string) => call<Calendar>("calendar_create", { name });

export const calendarDelete = (calendarId: string) => call<void>("calendar_delete", { calendarId });

export const icsSubscribe = (url: string) => call<Calendar>("ics_subscribe", { url });

export const icsImport = (text: string, name: string) =>
  call<{ calendar: Calendar; events: number }>("ics_import", { text, name });
