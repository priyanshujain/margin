import { useEffect, useRef, useState, type ReactNode } from "react";
import { useEscapeLayer } from "../hooks/escape";
import { registerCommands } from "../keys/commands";
import { Avatar } from "../ui/Avatar";
import { Button } from "../ui/Button";
import { Confirm, Sheet } from "../ui/Sheet";
import { PLUS } from "../icons";
import "./settings.css";

export interface SettingsSectionLink {
  id: string;
  label: string;
}

export interface SettingsShellProps {
  sections: readonly SettingsSectionLink[];
  active: string;
  onSelect: (id: string) => void;
  onClose: () => void;
  title?: string;
  children: ReactNode;
}

/** The whole stage: a rail of sections on the left and one section on the right. */
export function SettingsShell({
  sections,
  active,
  onSelect,
  onClose,
  title = "Settings",
  children,
}: SettingsShellProps) {
  useEscapeLayer(true, onClose);
  const rail = useRef<HTMLElement | null>(null);

  // The keyboard starts in settings rather than on whatever had it behind the stage.
  useEffect(() => {
    rail.current?.querySelector<HTMLElement>("[data-active]")?.focus();
  }, []);

  useEffect(() => {
    const step = (delta: number) => {
      const at = sections.findIndex((s) => s.id === active) + delta;
      const next = sections[Math.min(Math.max(at, 0), sections.length - 1)];
      if (next) onSelect(next.id);
    };
    return registerCommands({
      "select-next": () => step(1),
      "select-prev": () => step(-1),
    });
  }, [sections, active, onSelect]);

  return (
    <section className="settings" aria-label={title}>
      <nav className="settings-rail" aria-label={title} ref={rail}>
        <h1>{title}</h1>
        {sections.map((section) => (
          <button
            key={section.id}
            type="button"
            className="settings-tab"
            data-active={section.id === active ? "" : undefined}
            aria-current={section.id === active ? "page" : undefined}
            onClick={() => onSelect(section.id)}
          >
            {section.label}
          </button>
        ))}
      </nav>
      <div className="settings-panel">
        <div className="settings-inner">{children}</div>
      </div>
    </section>
  );
}

export interface SettingsSectionProps {
  title: string;
  note?: ReactNode;
  children?: ReactNode;
}

export function SettingsSection({ title, note, children }: SettingsSectionProps) {
  return (
    <>
      <h2 className="settings-title">{title}</h2>
      {note ? <p className="settings-note">{note}</p> : null}
      {children}
    </>
  );
}

export interface SettingRowProps {
  label: string;
  note?: ReactNode;
  children?: ReactNode;
}

export function SettingRow({ label, note, children }: SettingRowProps) {
  return (
    <div className="set-field">
      <div className="set-field-text">
        <span className="set-field-label">{label}</span>
        {note ? <span className="set-field-note">{note}</span> : null}
      </div>
      <div className="set-field-control">{children}</div>
    </div>
  );
}

export function SettingsQuiet({ children }: { children: ReactNode }) {
  return <p className="settings-quiet">{children}</p>;
}

export function SettingsWaiting({ children }: { children: ReactNode }) {
  return <div className="settings-waiting">{children}</div>;
}

export interface SettingsAccount {
  id: string;
  /** What the card leads with, such as the provider and the address. */
  name: string;
  /** The smaller line under it. */
  detail?: string;
  /** Feeds the avatar's initials and colour. */
  address?: string;
  hue?: number;
}

export interface AccountCardProps {
  account: SettingsAccount;
  trail?: ReactNode;
  children?: ReactNode;
}

export function AccountCard({ account, trail, children }: AccountCardProps) {
  return (
    <div className="acct">
      <div className="acct-head">
        <Avatar name={account.name} address={account.address ?? account.name} hue={account.hue} />
        <div>
          <div className="acct-name">{account.name}</div>
          {account.detail ? <div className="acct-addr">{account.detail}</div> : null}
        </div>
        {trail ?? <span />}
      </div>
      {children ? <div className="set-rows">{children}</div> : null}
    </div>
  );
}

export function SettingsFact({ label, children, wrap }: { label: string; children: ReactNode; wrap?: boolean }) {
  return (
    <div className="set-row" data-wrap={wrap ? "" : undefined}>
      <span className="lab">{label}</span>
      <span className="val">{children}</span>
    </div>
  );
}

export interface AccountsFootProps {
  onAdd: () => void;
  onRemove?: () => void;
  disabled?: boolean;
  addLabel?: string;
}

export function AccountsFoot({ onAdd, onRemove, disabled, addLabel = "Add account" }: AccountsFootProps) {
  return (
    <div className="acct-foot">
      <Button icon={PLUS} disabled={disabled} onClick={onAdd}>
        {addLabel}
      </Button>
      {onRemove ? (
        <Button variant="ghost" onClick={onRemove}>
          Remove an account
        </Button>
      ) : null}
    </div>
  );
}

/**
 * The wait while a provider's sign-in page is open in the browser. Closing the sheet in any way is
 * the same as Cancel, which must stop the app's listener so a late answer adds nothing.
 */
export interface OAuthStep {
  /** The provider as people know it, such as "Google". */
  provider: string;
  /** The consent link, once there is one. */
  link: string | null;
  /** Set when the sign-in failed; the step then offers Try again. */
  error?: string | null;
  /** A footnote about what signing in with this provider means. */
  note?: ReactNode;
  onReopen: () => void;
  onCopy: () => void;
  onRetry: () => void;
  onCancel: () => void;
}

export interface AddAccountSheetProps {
  open: boolean;
  title: string;
  onBack?: () => void;
  backLabel?: string;
  onClose: () => void;
  busy?: boolean;
  /** When set, the sheet shows the browser wait instead of its children. */
  oauth?: OAuthStep | null;
  children?: ReactNode;
}

export function AddAccountSheet({
  open,
  title,
  onBack,
  backLabel = "the providers",
  onClose,
  busy,
  oauth,
  children,
}: AddAccountSheetProps) {
  if (oauth) {
    return (
      <Sheet open={open} title={`Sign in with ${oauth.provider}`} onClose={oauth.onCancel}>
        <OAuthWait step={oauth} />
      </Sheet>
    );
  }
  return (
    <Sheet open={open} title={title} onBack={onBack} backLabel={backLabel} onClose={onClose} busy={busy}>
      {children}
    </Sheet>
  );
}

function OAuthWait({ step }: { step: OAuthStep }) {
  return (
    <div className="oauth-wait">
      {step.error ? (
        <p className="oauth-wait-error" role="alert">
          {step.error}
        </p>
      ) : (
        <p className="oauth-wait-line">
          Waiting for {step.provider} in your browser. Finish signing in there and this comes back on
          its own.
        </p>
      )}
      <div className="oauth-wait-actions">
        {step.error ? (
          <Button variant="primary" onClick={step.onRetry}>
            Try again
          </Button>
        ) : (
          <>
            <Button disabled={!step.link} onClick={step.onReopen}>
              Open link again
            </Button>
            <Button disabled={!step.link} onClick={step.onCopy}>
              Copy link
            </Button>
          </>
        )}
        <Button variant="ghost" onClick={step.onCancel}>
          Cancel
        </Button>
      </div>
      {step.note ? <p className="oauth-wait-note">{step.note}</p> : null}
    </div>
  );
}

export interface RemoveAccountSheetProps {
  open: boolean;
  accounts: readonly SettingsAccount[];
  onClose: () => void;
  /** What removing that account does, said before it happens. */
  consequence: (account: SettingsAccount) => ReactNode;
  onRemove: (account: SettingsAccount) => Promise<void>;
}

export function RemoveAccountSheet({ open, accounts, onClose, consequence, onRemove }: RemoveAccountSheetProps) {
  const [asking, setAsking] = useState<SettingsAccount | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) setAsking(null);
  }, [open]);

  const run = async (account: SettingsAccount) => {
    setBusy(true);
    try {
      await onRemove(account);
      setAsking(null);
      onClose();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Sheet open={open} title="Remove an account" busy={busy} onClose={onClose}>
      {asking ? (
        <Confirm
          title={`Remove ${asking.name}?`}
          body={consequence(asking)}
          confirmLabel="Remove"
          busy={busy}
          busyLabel="Removing"
          onConfirm={() => void run(asking)}
          onCancel={() => setAsking(null)}
        />
      ) : (
        accounts.map((account) => (
          <div className="remove-row" key={account.id}>
            <Avatar name={account.name} address={account.address ?? account.name} hue={account.hue} size="sm" />
            <span className="remove-who">{account.name}</span>
            <Button size="sm" variant="danger" onClick={() => setAsking(account)}>
              Remove
            </Button>
          </div>
        ))
      )}
    </Sheet>
  );
}
