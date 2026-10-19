// Settings on the stage, in margin-core's shell: external accounts, the calendars the grid draws,
// and the few preferences that are genuinely global. Accounts are provider-neutral: one Add account
// flow asks which provider, then either hands over to that provider's sign-in page or asks for a
// username and an app password.

import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import {
  AccountCard,
  AccountsFoot,
  AddAccountSheet,
  RemoveAccountSheet,
  SettingRow,
  SettingsFact,
  SettingsSection,
  SettingsShell,
  type SettingsAccount,
} from "@margin/core/settings";
import { Button, Confirm, Icon, NO_AUTOFILL, Segment } from "@margin/core/ui";
import { LOCAL_ACCOUNT, SUBSCRIBED_ACCOUNT, type Account, type Calendar } from "../ipc";
import { useAccounts } from "../store/useAccounts";
import { useCalendarView } from "../store/useCalendarView";
import { useSettingsScreen, type SettingsSectionId } from "../store/useSettingsScreen";
import { useTheme } from "../store/useTheme";
import { setWeekStartDay, weekStartDay } from "../time";
import { calSlot } from "./GridModel";
import { roleLabel } from "./overlayModel";

const SECTIONS: { id: SettingsSectionId; label: string }[] = [
  { id: "accounts", label: "Accounts" },
  { id: "calendars", label: "Calendars" },
  { id: "general", label: "General" },
];

export function SettingsScreen() {
  const section = useSettingsScreen((s) => s.section);
  const select = useSettingsScreen((s) => s.select);
  const close = useSettingsScreen((s) => s.close);
  const refresh = useAccounts((s) => s.refresh);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <SettingsShell
      sections={SECTIONS}
      active={section}
      onSelect={(id) => select(id as SettingsSectionId)}
      onClose={close}
      back={{ label: "Back to Calendar", onBack: close }}
    >
      {section === "accounts" ? <AccountsSection /> : null}
      {section === "calendars" ? <CalendarsSection /> : null}
      {section === "general" ? <GeneralSection /> : null}
    </SettingsShell>
  );
}

// -- accounts ----------------------------------------------------------------------------------

interface Provider {
  id: string;
  label: string;
  /** Google signs in on its own page; the rest take a username and an app password. */
  oauth: boolean;
  /** Prefilled for a service whose CalDAV address is the same for everybody. */
  server?: string;
  username?: string;
}

const PROVIDERS: Provider[] = [
  { id: "google", label: "Google", oauth: true },
  { id: "icloud", label: "iCloud", oauth: false, server: "https://caldav.icloud.com", username: "Apple Account email" },
  { id: "fastmail", label: "Fastmail", oauth: false, server: "https://caldav.fastmail.com", username: "Fastmail address" },
  { id: "caldav", label: "CalDAV server", oauth: false },
];

function hostOf(url: string | null | undefined): string | null {
  if (!url) return null;
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/** The provider an account is named after: the service when its server says which, else the host. */
export function providerName(account: Account): string {
  if (account.provider === "google") return "Google";
  const host = hostOf(account.server) ?? "";
  if (/(^|\.)icloud\.com$/i.test(host)) return "iCloud";
  if (/(^|\.)fastmail\.com$/i.test(host)) return "Fastmail";
  return host || "Server";
}

export function accountTitle(account: Account): string {
  return `${providerName(account)}, ${account.email}`;
}

const COUNTS = ["No", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight"];

function AccountsSection() {
  const accounts = useAccounts((s) => s.accounts);
  const calendars = useAccounts((s) => s.calendars);
  const phase = useAccounts((s) => s.phase);
  const disconnect = useAccounts((s) => s.disconnect);

  const [adding, setAdding] = useState(false);
  const [removing, setRemoving] = useState(false);
  const waiting = phase === "connecting";

  const countFor = (id: string) => calendars.filter((c) => c.accountId === id).length;
  const cards: (SettingsAccount & { source: Account })[] = accounts.map((account) => ({
    id: account.id,
    name: accountTitle(account),
    address: account.email,
    detail: account.connected
      ? `${countFor(account.id)} calendar${countFor(account.id) === 1 ? "" : "s"}`
      : "Needs reconnecting",
    source: account,
  }));
  const many = accounts.length;
  const said = many < COUNTS.length ? COUNTS[many] : String(many);

  return (
    <SettingsSection
      title="Accounts"
      note={
        many === 0
          ? "No external accounts. Calendars on this device work without one."
          : `${said} external ${many === 1 ? "account" : "accounts"}, each bringing its own calendars.`
      }
    >
      {cards.map((card) => (
        <AccountCard key={card.id} account={card}>
          {card.source.provider === "google" || providerName(card.source) === hostOf(card.source.server) ? null : (
            <SettingsFact label="Server">{hostOf(card.source.server) ?? "Unknown"}</SettingsFact>
          )}
        </AccountCard>
      ))}

      <AccountsFoot
        disabled={waiting || phase === "working"}
        onAdd={() => setAdding(true)}
        onRemove={many > 0 ? () => setRemoving(true) : undefined}
      />

      <AddAccount open={adding} onClose={() => setAdding(false)} />

      <RemoveAccountSheet
        open={removing}
        accounts={cards}
        onClose={() => setRemoving(false)}
        consequence={(card) => (
          <p>
            {cards.find((c) => c.id === card.id)?.source.provider === "google"
              ? "This hands Margin's grant back to Google and deletes this account's calendars, events and pending changes from this computer. Nothing changes in the account itself."
              : "This forgets the app password and deletes this account's calendars, events and pending changes from this computer. Nothing changes on the server."}
          </p>
        )}
        onRemove={async (card) => {
          await disconnect(card.id);
          void useCalendarView.getState().load();
        }}
      />
    </SettingsSection>
  );
}

function AddAccount({ open, onClose }: { open: boolean; onClose: () => void }) {
  const phase = useAccounts((s) => s.phase);
  const error = useAccounts((s) => s.error);
  const authUrl = useAccounts((s) => s.authUrl);
  const connect = useAccounts((s) => s.connect);
  const cancelConnect = useAccounts((s) => s.cancelConnect);
  const openAuthUrl = useAccounts((s) => s.openAuthUrl);
  const copyAuthUrl = useAccounts((s) => s.copyAuthUrl);
  const connectCalDav = useAccounts((s) => s.connectCalDav);
  const [step, setStep] = useState<"provider" | "login" | "oauth">("provider");
  const [providerId, setProviderId] = useState(PROVIDERS[0].id);
  const [server, setServer] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const provider = PROVIDERS.find((p) => p.id === providerId) ?? PROVIDERS[0];
  const busy = phase === "working";
  const login = useRef<HTMLFormElement | null>(null);

  // The sheet focuses its first field when it opens, not when it moves to its second step.
  useEffect(() => {
    if (step === "login") login.current?.querySelector<HTMLElement>("input")?.focus();
  }, [step]);

  useEffect(() => {
    if (!open) return;
    setStep("provider");
    setServer("");
    setUsername("");
    setPassword("");
  }, [open]);

  // The browser has the flow from here; the sheet waits for it and closes once an account lands.
  const signInWithProvider = () => {
    setStep("oauth");
    void connect().then((ok) => {
      if (ok) onClose();
    });
  };

  const proceed = (e: FormEvent) => {
    e.preventDefault();
    if (provider.oauth) signInWithProvider();
    else setStep("login");
  };

  // Closing the sheet while the browser has the flow is the same as Cancel.
  const cancelSignIn = () => {
    cancelConnect();
    onClose();
  };

  const signIn = async (e: FormEvent) => {
    e.preventDefault();
    const address = provider.server ?? server;
    if (!address.trim() || !username.trim() || !password) return;
    if (await connectCalDav(address, username, password)) {
      onClose();
      void useCalendarView.getState().load();
    }
  };

  return (
    <AddAccountSheet
      open={open}
      oauth={
        step === "oauth"
          ? {
              provider: provider.label,
              link: authUrl,
              error: phase === "error" ? error : null,
              note: "Margin Calendar, Margin and Margin Mail share one Google client, so your Google account lists them once, as Margin, and revoking it revokes all three.",
              onReopen: openAuthUrl,
              onCopy: () => void copyAuthUrl(),
              onRetry: signInWithProvider,
              onCancel: cancelSignIn,
            }
          : null
      }
      title={step === "provider" ? "Add account" : provider.label}
      onBack={step === "login" && !busy ? () => setStep("provider") : undefined}
      onClose={onClose}
      busy={busy}
    >
      {step === "provider" ? (
        <form className="add-account" onSubmit={proceed}>
          <label className="field">
            <span className="field-label">Provider</span>
            <select
              className="field-input"
              value={providerId}
              data-autofocus=""
              onChange={(e) => setProviderId(e.target.value)}
            >
              {PROVIDERS.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
            </select>
          </label>
          <p className="panel-note">
            {provider.oauth
              ? `${provider.label} asks you to sign in on its own page in your browser.`
              : "Signs in with your username and an app password made for Margin Calendar at the provider."}
          </p>
          <div className="add-account-actions">
            <Button type="submit" variant="primary">
              Continue
            </Button>
          </div>
        </form>
      ) : (
        <form className="add-account" ref={login} onSubmit={(e) => void signIn(e)}>
          {provider.server ? null : (
            <label className="field">
              <span className="field-label">Server</span>
              <input
                className="field-input"
                type="url"
                value={server}
                placeholder="https://calendar.example.com"
                {...NO_AUTOFILL}
                onChange={(e) => setServer(e.target.value)}
              />
            </label>
          )}
          <label className="field">
            <span className="field-label">{provider.username ?? "Username"}</span>
            <input
              className="field-input"
              type="text"
              value={username}
              {...NO_AUTOFILL}
              onChange={(e) => setUsername(e.target.value)}
            />
          </label>
          <label className="field">
            <span className="field-label">App password</span>
            <input
              className="field-input"
              type="password"
              value={password}
              {...NO_AUTOFILL}
              onChange={(e) => setPassword(e.target.value)}
            />
          </label>
          {phase === "error" && error ? <p className="panel-note" data-error="">{error}</p> : null}
          <div className="add-account-actions">
            <Button
              type="submit"
              variant="primary"
              disabled={busy || !(provider.server ?? server).trim() || !username.trim() || !password}
            >
              {busy ? "Signing in…" : "Sign in"}
            </Button>
          </div>
        </form>
      )}
    </AddAccountSheet>
  );
}

// -- calendars ---------------------------------------------------------------------------------

const TRASH = "M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6";

interface Group {
  accountId: string;
  label: string;
  calendars: Calendar[];
}

function CalendarsSection() {
  const accounts = useAccounts((s) => s.accounts);
  const calendars = useAccounts((s) => s.calendars);
  const setSelected = useAccounts((s) => s.setSelected);
  const createCalendar = useAccounts((s) => s.createCalendar);
  const deleteCalendar = useAccounts((s) => s.deleteCalendar);
  const subscribe = useAccounts((s) => s.subscribe);
  const importFile = useAccounts((s) => s.importFile);
  const [name, setName] = useState("");
  const [feed, setFeed] = useState("");
  const [busy, setBusy] = useState(false);
  const [doomed, setDoomed] = useState<Calendar | null>(null);
  const picker = useRef<HTMLInputElement | null>(null);

  const groups = useMemo((): Group[] => {
    const titles = new Map(accounts.map((a) => [a.id, accountTitle(a)]));
    const found = new Map<string, Group>();
    for (const calendar of calendars) {
      const label =
        calendar.accountId === LOCAL_ACCOUNT
          ? "On this device"
          : calendar.accountId === SUBSCRIBED_ACCOUNT
            ? "Subscriptions"
            : (titles.get(calendar.accountId) ?? "Disconnected account");
      const group = found.get(calendar.accountId) ?? { accountId: calendar.accountId, label, calendars: [] };
      group.calendars.push(calendar);
      found.set(calendar.accountId, group);
    }
    const rank = (g: Group) => (g.accountId === LOCAL_ACCOUNT ? 0 : g.accountId === SUBSCRIBED_ACCOUNT ? 2 : 1);
    return [...found.values()].sort((a, b) => rank(a) - rank(b) || a.label.localeCompare(b.label));
  }, [accounts, calendars]);

  const reload = () => void useCalendarView.getState().load();

  const add = async (e: FormEvent) => {
    e.preventDefault();
    if (name.trim() && (await createCalendar(name))) setName("");
  };

  const follow = async (e: FormEvent) => {
    e.preventDefault();
    if (!feed.trim() || busy) return;
    setBusy(true);
    if (await subscribe(feed)) setFeed("");
    setBusy(false);
    reload();
  };

  const pick = async (file: File | undefined) => {
    if (!file) return;
    await importFile(file);
    if (picker.current) picker.current.value = "";
    reload();
  };

  return (
    <SettingsSection title="Calendars" note="Which calendars the grid draws, and the ones that live on this device.">
      {doomed ? (
        <Confirm
          title={
            doomed.accountId === SUBSCRIBED_ACCOUNT
              ? `Unsubscribe from “${doomed.summary || "Untitled"}”?`
              : `Delete “${doomed.summary || "Untitled"}”?`
          }
          body={
            <p>
              {doomed.accountId === SUBSCRIBED_ACCOUNT
                ? "Its events leave this device. The feed itself is not touched, and you can subscribe again."
                : "Every event in it goes too. It lives only on this device, so nothing else has a copy."}
            </p>
          }
          confirmLabel={doomed.accountId === SUBSCRIBED_ACCOUNT ? "Unsubscribe" : "Delete"}
          onConfirm={() => {
            const id = doomed.id;
            setDoomed(null);
            void deleteCalendar(id).then(reload);
          }}
          onCancel={() => setDoomed(null)}
        />
      ) : null}

      {groups.map((group) => (
        <div className="cal-group" key={group.accountId}>
          <span className="field-label">{group.label}</span>
          {group.calendars.map((calendar) => {
            const role = roleLabel(calendar.accessRole);
            const removable =
              calendar.accountId === LOCAL_ACCOUNT || calendar.accountId === SUBSCRIBED_ACCOUNT;
            return (
              <label className="cal-row" key={calendar.id} title={calendar.description ?? undefined}>
                <input
                  type="checkbox"
                  className="cal-check"
                  checked={calendar.selected}
                  onChange={() => {
                    void setSelected(calendar.id, !calendar.selected);
                    reload();
                  }}
                />
                <span
                  className="cal-dot"
                  style={{ background: `var(--hue-${calSlot(calendar.colorHex, calendar.id)})` }}
                />
                <span className="cal-name">{calendar.summary || "Untitled"}</span>
                {role ? <span className="cal-role">{role}</span> : null}
                {removable ? (
                  <button
                    type="button"
                    className="icon-button cal-delete"
                    title={`Delete ${calendar.summary || "this calendar"}`}
                    aria-label={`Delete ${calendar.summary || "this calendar"}`}
                    onClick={(e) => {
                      e.preventDefault();
                      setDoomed(calendar);
                    }}
                  >
                    <Icon d={TRASH} size={14} />
                  </button>
                ) : null}
              </label>
            );
          })}
        </div>
      ))}

      <SettingRow label="New calendar" note="Lives on this device. Nothing else has a copy.">
        <form className="cal-new" onSubmit={(e) => void add(e)}>
          <input
            className="field-input"
            type="text"
            value={name}
            placeholder="Name"
            aria-label="New calendar name"
            {...NO_AUTOFILL}
            onChange={(e) => setName(e.target.value)}
          />
          <Button type="submit" disabled={!name.trim()}>
            Add
          </Button>
        </form>
      </SettingRow>

      <SettingRow label="Subscribe" note="A read-only feed by its address, refreshed on every sync.">
        <form className="cal-new" onSubmit={(e) => void follow(e)}>
          <input
            className="field-input"
            type="url"
            value={feed}
            placeholder="https:// or webcal://"
            aria-label="Calendar address to subscribe to"
            {...NO_AUTOFILL}
            onChange={(e) => setFeed(e.target.value)}
          />
          <Button type="submit" disabled={!feed.trim() || busy}>
            {busy ? "Subscribing…" : "Subscribe"}
          </Button>
        </form>
      </SettingRow>

      <SettingRow label="Import" note="An .ics file, read into a new calendar on this device.">
        <input
          ref={picker}
          type="file"
          accept=".ics,.ical,text/calendar"
          hidden
          onChange={(e) => void pick(e.target.files?.[0])}
        />
        <Button onClick={() => picker.current?.click()}>Choose a file…</Button>
      </SettingRow>
    </SettingsSection>
  );
}

// -- general -----------------------------------------------------------------------------------

function GeneralSection() {
  const theme = useTheme((s) => s.theme);
  const selectTheme = useTheme((s) => s.select);
  const [start, setStart] = useState<0 | 1>(() => (weekStartDay() === 0 ? 0 : 1));

  const chooseStart = (day: 0 | 1) => {
    if (day === start) return;
    setWeekStartDay(day);
    setStart(day);
    useCalendarView.getState().setAnchor(useCalendarView.getState().anchor);
  };

  return (
    <SettingsSection title="General">
      <SettingRow label="Theme">
        <Segment
          label="Theme"
          options={[
            { id: "light", label: "Light" },
            { id: "dark", label: "Dark" },
          ]}
          value={theme}
          onChange={(id) => selectTheme(id as "light" | "dark")}
        />
      </SettingRow>
      <SettingRow
        label="Week starts on"
        note="Week view begins on this day rather than on today. The arrows and h and l still slide it a day at a time when you want a look either side."
      >
        <Segment
          label="Week starts on"
          options={[
            { id: "0", label: "Sunday" },
            { id: "1", label: "Monday" },
          ]}
          value={String(start)}
          onChange={(id) => chooseStart(id === "0" ? 0 : 1)}
        />
      </SettingRow>
    </SettingsSection>
  );
}

export default SettingsScreen;
