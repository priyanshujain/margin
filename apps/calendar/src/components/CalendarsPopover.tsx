// The calendar icon in the title bar: which calendars the grid draws, grouped by where they live,
// and the three ways to get another. Detailed management stays in Settings > Calendars, one link
// away at the foot.

import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { create } from "zustand";
import { Button, NO_AUTOFILL, Popover } from "@margin/core/ui";
import { LOCAL_ACCOUNT, SUBSCRIBED_ACCOUNT, type Calendar } from "../ipc";
import { useAccounts } from "../store/useAccounts";
import { useCalendarView } from "../store/useCalendarView";
import { useSettingsScreen } from "../store/useSettingsScreen";
import { calSlot } from "./GridModel";
import { Sheet } from "./overlayShell";
import { accountTitle } from "./SettingsScreen";

type Action = "new" | "subscribe" | "import";

interface PopoverState {
  open: boolean;
  action: Action | null;
  toggle: () => void;
  close: () => void;
  start: (action: Action) => void;
  finish: () => void;
}

export const useCalendarsPopover = create<PopoverState>((set) => ({
  open: false,
  action: null,
  toggle: () => set((s) => ({ open: !s.open })),
  close: () => set({ open: false }),
  start: (action) => set({ open: false, action }),
  finish: () => set({ action: null }),
}));

interface Group {
  key: string;
  label: string;
  calendars: Calendar[];
}

function useGroups(): Group[] {
  const accounts = useAccounts((s) => s.accounts);
  const calendars = useAccounts((s) => s.calendars);
  return useMemo(() => {
    const titles = new Map(accounts.map((a) => [a.id, accountTitle(a)]));
    const found = new Map<string, Group>();
    for (const calendar of calendars) {
      const label =
        calendar.accountId === LOCAL_ACCOUNT
          ? "On this device"
          : calendar.accountId === SUBSCRIBED_ACCOUNT
            ? "Subscriptions"
            : (titles.get(calendar.accountId) ?? "Disconnected account");
      const group = found.get(calendar.accountId) ?? { key: calendar.accountId, label, calendars: [] };
      group.calendars.push(calendar);
      found.set(calendar.accountId, group);
    }
    const rank = (g: Group) => (g.key === LOCAL_ACCOUNT ? 0 : g.key === SUBSCRIBED_ACCOUNT ? 2 : 1);
    return [...found.values()].sort((a, b) => rank(a) - rank(b) || a.label.localeCompare(b.label));
  }, [accounts, calendars]);
}

export function CalendarsPopover({ anchor }: { anchor: HTMLElement | null }) {
  const open = useCalendarsPopover((s) => s.open);
  const close = useCalendarsPopover((s) => s.close);
  const start = useCalendarsPopover((s) => s.start);
  const setSelected = useAccounts((s) => s.setSelected);
  const refresh = useAccounts((s) => s.refresh);
  const groups = useGroups();

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  return (
    <Popover open={open} anchor={anchor} onClose={close} placement="bottom-end" width={300} label="Calendars">
      <div className="cal-pop">
        {groups.length === 0 ? <p className="cal-pop-empty">No calendars yet.</p> : null}
        {groups.map((group) => (
          <div className="cal-group" key={group.key}>
            <span className="field-label">{group.label}</span>
            {group.calendars.map((calendar) => (
              <label className="cal-row" key={calendar.id}>
                <input
                  type="checkbox"
                  className="cal-check"
                  checked={calendar.selected}
                  onChange={() => {
                    void setSelected(calendar.id, !calendar.selected);
                    void useCalendarView.getState().load();
                  }}
                />
                <span
                  className="cal-dot"
                  style={{ background: `var(--hue-${calSlot(calendar.colorHex, calendar.id)})` }}
                />
                <span className="cal-name">{calendar.summary || "Untitled"}</span>
              </label>
            ))}
          </div>
        ))}
        <div className="cal-pop-actions">
          <button type="button" className="cal-pop-action" onClick={() => start("new")}>
            New calendar…
          </button>
          <button type="button" className="cal-pop-action" onClick={() => start("subscribe")}>
            Subscribe to a calendar…
          </button>
          <button type="button" className="cal-pop-action" onClick={() => start("import")}>
            Import a file…
          </button>
        </div>
        <button
          type="button"
          className="cal-pop-link"
          onClick={() => {
            close();
            useSettingsScreen.getState().show("calendars");
          }}
        >
          Calendar settings
        </button>
      </div>
    </Popover>
  );
}

/** The three small popups the popover's actions open. */
export function CalendarActionSheets() {
  const action = useCalendarsPopover((s) => s.action);
  const finish = useCalendarsPopover((s) => s.finish);
  const createCalendar = useAccounts((s) => s.createCalendar);
  const subscribe = useAccounts((s) => s.subscribe);
  const importFile = useAccounts((s) => s.importFile);
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const picker = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    setValue("");
    setBusy(false);
  }, [action]);

  const done = (ok: boolean) => {
    setBusy(false);
    if (ok) {
      finish();
      void useCalendarView.getState().load();
    }
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!value.trim() || busy) return;
    setBusy(true);
    done(action === "new" ? await createCalendar(value) : await subscribe(value));
  };

  if (action === "import") {
    return (
      <Sheet open title="Import a file" onClose={finish}>
        <p className="panel-note">An .ics file is read into a new calendar on this device.</p>
        <input
          ref={picker}
          type="file"
          accept=".ics,.ical,text/calendar"
          hidden
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (!file) return;
            setBusy(true);
            void importFile(file).then(() => done(true));
          }}
        />
        <div className="add-account-actions">
          <Button variant="primary" disabled={busy} onClick={() => picker.current?.click()}>
            {busy ? "Importing…" : "Choose a file…"}
          </Button>
        </div>
      </Sheet>
    );
  }

  const subscribing = action === "subscribe";
  return (
    <Sheet
      open={action !== null}
      title={subscribing ? "Subscribe to a calendar" : "New calendar"}
      onClose={finish}
    >
      <form className="add-account" onSubmit={(e) => void submit(e)}>
        <label className="field">
          <span className="field-label">{subscribing ? "Address" : "Name"}</span>
          <input
            className="field-input"
            type={subscribing ? "url" : "text"}
            value={value}
            placeholder={subscribing ? "https:// or webcal://" : "Calendar"}
            data-autofocus=""
            {...NO_AUTOFILL}
            onChange={(e) => setValue(e.target.value)}
          />
        </label>
        <p className="panel-note">
          {subscribing
            ? "A read-only feed, refreshed on every sync."
            : "Lives on this device. Nothing else has a copy."}
        </p>
        <div className="add-account-actions">
          <Button type="submit" variant="primary" disabled={!value.trim() || busy}>
            {subscribing ? (busy ? "Subscribing…" : "Subscribe") : "Create"}
          </Button>
        </div>
      </form>
    </Sheet>
  );
}
