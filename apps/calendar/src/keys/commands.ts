// What every command id in `bindings.ts` does, registered with margin-core's command engine.
//
// A key, a menu item and a palette row all end up here: the native menu emits an id and that id is
// a command, not a second code path. Labels live on the bindings, so the sheet, the palette and the
// menu cannot describe the same thing in three different ways. Side effects go through the stores
// and through `gridCommands`, the imperative surface the grid publishes for exactly this.

import { openUrl } from "@tauri-apps/plugin-opener";
import { eventDelete } from "../api/events";
import { gridCommands } from "../components/GridStore";
import { useEditor } from "../components/useEditor";
import type { Instance } from "../ipc";
import { defaultStart } from "../palette/parse";
import { eventTitle } from "../components/GridModel";
import { useCalendarView } from "../store/useCalendarView";
import { useOverlays } from "../store/useOverlays";
import { useSettingsScreen } from "../store/useSettingsScreen";
import { useCalendarsPopover } from "../components/CalendarsPopover";
import { useSync } from "../store/useSync";
import { useTheme } from "../store/useTheme";
import { notify } from "@margin/core/hooks";
import { minutesFromMidnight, startOfDay } from "../time";
import { checkForUpdates } from "./updates";
import { registerCommands, type Handler } from "@margin/core/keys";
import type { CommandId } from "./bindings";

const ISSUES_URL = "https://github.com/priyanshujain/margin-calendar/issues";

/** New events start an hour long, whether they are dragged out or typed. */
const NEW_MINUTES = 60;

const view = () => useCalendarView.getState();
const overlays = () => useOverlays.getState();
const settingsScreen = () => useSettingsScreen.getState();

function selectedInstance(): Instance | null {
  const { instances, selected } = view();
  if (!selected) return null;
  return (
    instances.find(
      (i) => i.eventId === selected.eventId && i.originalStart === selected.originalStart,
    ) ?? null
  );
}

function openSelection(): void {
  const instance = selectedInstance();
  // Enter on nothing at all is a request for the first event of the day, not a mistake.
  if (!instance) {
    view().selectAdjacent(1);
    return;
  }
  useEditor.getState().edit({ eventId: instance.eventId, originalStart: instance.originalStart });
}

function editSelection(): void {
  const instance = selectedInstance();
  if (!instance) {
    notify("Nothing is selected");
    return;
  }
  useEditor.getState().edit({ eventId: instance.eventId, originalStart: instance.originalStart });
}

/**
 * `this` is the only scope a keystroke may pick: deleting a whole series is a decision that
 * belongs in the editor, behind a confirmation.
 */
async function deleteSelection(): Promise<void> {
  const instance = selectedInstance();
  if (!instance) {
    notify("Nothing is selected");
    return;
  }
  if (instance.readOnly) {
    notify("That calendar is read only");
    return;
  }
  const title = eventTitle(instance);
  try {
    await eventDelete(
      { eventId: instance.eventId, originalStart: instance.originalStart },
      "this",
    );
    view().select(null);
    await view().load();
    notify(instance.recurring ? `Deleted this occurrence of “${title}”` : `Deleted “${title}”`);
  } catch (e) {
    notify(`Could not delete “${title}”: ${e}`);
  }
}

/** The grid takes a typed-in-place draft when the day is on screen; the agenda has no column. */
function newEvent(): void {
  const startMs = defaultStart(view().anchor, Date.now());
  const dayStart = startOfDay(startMs);
  if (view().view !== "agenda" && gridCommands.days().includes(dayStart)) {
    gridCommands.draftOn(dayStart, Math.round(minutesFromMidnight(startMs)), NEW_MINUTES);
    return;
  }
  useEditor.getState().create({ startMs, endMs: startMs + NEW_MINUTES * 60_000 });
}

const HANDLERS: Record<CommandId, Handler> = {
  "day-prev": () => view().moveDay(-1),
  "day-next": () => view().moveDay(1),
  "week-prev": () => view().moveWeek(-1),
  "week-next": () => view().moveWeek(1),
  today: () => view().goToday(),
  "select-next": () => view().selectAdjacent(1),
  "select-prev": () => view().selectAdjacent(-1),
  "mini-month": () => overlays().show("mini-month"),

  "view-day": () => view().setView("day"),
  "view-week": () => view().setView("week"),
  "view-agenda": () => view().setView("agenda"),
  fold: () => gridCommands.toggleFold(),
  "unfold-all": () => gridCommands.unfoldAll(),

  "new-event": newEvent,
  "open-selection": openSelection,
  "edit-selection": editSelection,
  "delete-selection": () => {
    void deleteSelection();
  },

  search: () => overlays().show("search"),
  "command-palette": () => overlays().show("palette"),
  shortcuts: () => overlays().show("shortcuts"),

  "sync-now": () => {
    void useSync.getState().run();
  },
  // The title bar's popover where there is a title bar; a phone has none, so Settings instead.
  calendars: () => {
    if (document.querySelector("[data-calendars-button]")) useCalendarsPopover.getState().toggle();
    else settingsScreen().show("calendars");
  },
  accounts: () => settingsScreen().show("accounts"),
  settings: () => (settingsScreen().open ? settingsScreen().close() : settingsScreen().show()),
  "toggle-theme": () => useTheme.getState().toggle(),
  "check-updates": () => {
    void checkForUpdates();
  },
  "report-issue": () => {
    openUrl(ISSUES_URL).catch(() => notify("Could not open the browser"));
  },
};

registerCommands(HANDLERS);
