import { useEffect, useState, type FormEvent } from "react";
import { getVersion } from "@tauri-apps/api/app";
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
import { Button, Confirm, Segment, Toggle } from "@margin/core/ui";
import { schemeOf, type ThemeChoice } from "@margin/core/hooks";
import { useProofing } from "@margin/core/editor/base";
import { formatBackupTime, gdriveListBackups } from "../backup";
import { isDesktop } from "../ipc";
import { checkForUpdates } from "../updater";
import { useBackup } from "../store/useBackup";
import { useSettingsScreen, type SettingsSectionId } from "../store/useSettingsScreen";
import { useTheme } from "../store/useTheme";

const SECTIONS: { id: SettingsSectionId; label: string }[] = [
  { id: "appearance", label: "Appearance" },
  { id: "writing", label: "Writing" },
  { id: "backup", label: "Backup" },
  { id: "about", label: "About" },
];

export function SettingsScreen() {
  const open = useSettingsScreen((s) => s.open);
  const section = useSettingsScreen((s) => s.section);
  const select = useSettingsScreen((s) => s.select);
  const close = useSettingsScreen((s) => s.close);

  if (!open) return null;
  return (
    <SettingsShell
      sections={SECTIONS}
      active={section}
      onSelect={(id) => select(id as SettingsSectionId)}
      onClose={close}
      back={{ label: "Back to Margin", onBack: close }}
    >
      {section === "appearance" ? <AppearanceSection /> : null}
      {section === "writing" ? <WritingSection /> : null}
      {section === "backup" ? <BackupSection /> : null}
      {section === "about" ? <AboutSection /> : null}
    </SettingsShell>
  );
}

function AppearanceSection() {
  const choice = useTheme((s) => s.choice);
  const palettes = useTheme((s) => s.palettes);
  const selectTheme = useTheme((s) => s.select);
  const options = [
    { id: "system", label: "System" },
    ...palettes.map((id) => ({ id, label: schemeOf(id) === "dark" ? "Dark" : "Light" })),
  ];
  return (
    <SettingsSection title="Appearance">
      <SettingRow label="Theme" note="System follows your computer's light and dark setting.">
        <Segment
          label="Theme"
          options={options}
          value={choice}
          onChange={(id) => selectTheme(id as ThemeChoice)}
        />
      </SettingRow>
    </SettingsSection>
  );
}

function WritingSection() {
  const enabled = useProofing((s) => s.enabled);
  const grammar = useProofing((s) => s.grammar);
  const setEnabled = useProofing((s) => s.setEnabled);
  const setGrammar = useProofing((s) => s.setGrammar);
  return (
    <SettingsSection title="Writing" note="Checks run on your computer and only for books set in English.">
      <SettingRow label="Check spelling">
        <Toggle checked={enabled} onChange={setEnabled} label="Check spelling" />
      </SettingRow>
      <SettingRow label="Check grammar">
        <Toggle checked={grammar} onChange={setGrammar} label="Check grammar" />
      </SettingRow>
    </SettingsSection>
  );
}

function BackupSection() {
  const connected = useBackup((s) => s.connected);
  const email = useBackup((s) => s.email);
  const lastBackup = useBackup((s) => s.lastBackup);
  const pending = useBackup((s) => s.pending);
  const needsReauth = useBackup((s) => s.needsReauth);
  const phase = useBackup((s) => s.phase);
  const refresh = useBackup((s) => s.refresh);
  const disconnect = useBackup((s) => s.disconnect);
  const backup = useBackup((s) => s.backup);
  const sync = useBackup((s) => s.sync);
  const restore = useBackup((s) => s.restore);

  const [adding, setAdding] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [confirmRestore, setConfirmRestore] = useState(false);
  const [remoteCount, setRemoteCount] = useState<number | null>(null);
  const working = phase === "working";

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!connected || needsReauth) {
      setRemoteCount(null);
      return;
    }
    let active = true;
    gdriveListBackups()
      .then((list) => active && setRemoteCount(list.length))
      .catch(() => active && setRemoteCount(null));
    return () => {
      active = false;
    };
  }, [connected, needsReauth, lastBackup]);

  const cards: SettingsAccount[] = connected
    ? [
        {
          id: "gdrive",
          name: "Google Drive",
          address: email ?? undefined,
          detail: needsReauth ? "Needs reconnecting" : pending ? "Changes not backed up" : formatBackupTime(lastBackup),
        },
      ]
    : [];

  return (
    <SettingsSection
      title="Backup"
      note={
        connected
          ? "Syncs on launch and right after connecting, backs up when you close Margin, and every 15 minutes while there are changes."
          : "No external accounts. Your projects live on this computer; connect a storage account to keep a private backup."
      }
    >
      {cards.map((card) => (
        <AccountCard key={card.id} account={card}>
          <SettingsFact label="Folder">margin</SettingsFact>
          {needsReauth ? (
            <p className="settings-warn">
              Margin can't reach its Drive folder: the permission to create and edit its own files was not granted.
              Remove the account and add it again, and tick that box on the Google screen.
            </p>
          ) : (
            <div className="settings-actions">
              <Button variant="primary" disabled={working} onClick={() => void sync()}>
                {working ? "Working…" : "Sync now"}
              </Button>
              <Button disabled={working} onClick={() => void backup()}>
                Back up only
              </Button>
              <Button disabled={working || remoteCount === 0} onClick={() => setConfirmRestore(true)}>
                {remoteCount === null ? "Restore from Drive" : `Restore ${remoteCount} file${remoteCount === 1 ? "" : "s"}`}
              </Button>
            </div>
          )}
        </AccountCard>
      ))}

      <AccountsFoot
        disabled={phase === "connecting" || working || connected}
        onAdd={() => setAdding(true)}
        onRemove={connected ? () => setRemoving(true) : undefined}
      />

      <AddStorageAccount open={adding} onClose={() => setAdding(false)} />

      <RemoveAccountSheet
        open={removing}
        accounts={cards}
        onClose={() => setRemoving(false)}
        consequence={() => (
          <p>
            This hands Margin's grant back to Google and stops backing up. The copies already in the margin folder on
            Drive stay there, and nothing on this computer is deleted.
          </p>
        )}
        onRemove={async () => {
          await disconnect();
        }}
      />

      {confirmRestore && (
        <Confirm
          title="Restore from Google Drive"
          body="This replaces your local projects with the copies in Google Drive. Any local changes that haven't been backed up will be lost."
          confirmLabel="Restore"
          variant="danger"
          onConfirm={() => {
            setConfirmRestore(false);
            void restore();
          }}
          onCancel={() => setConfirmRestore(false)}
        />
      )}
    </SettingsSection>
  );
}

const PROVIDERS = [{ id: "gdrive", label: "Google Drive", provider: "Google" }];

function AddStorageAccount({ open, onClose }: { open: boolean; onClose: () => void }) {
  const phase = useBackup((s) => s.phase);
  const error = useBackup((s) => s.error);
  const authUrl = useBackup((s) => s.authUrl);
  const connect = useBackup((s) => s.connect);
  const cancelConnect = useBackup((s) => s.cancelConnect);
  const openAuthUrl = useBackup((s) => s.openAuthUrl);
  const copyAuthUrl = useBackup((s) => s.copyAuthUrl);
  const [step, setStep] = useState<"provider" | "oauth">("provider");
  const [providerId, setProviderId] = useState(PROVIDERS[0].id);
  const provider = PROVIDERS.find((p) => p.id === providerId) ?? PROVIDERS[0];

  useEffect(() => {
    if (open) setStep("provider");
  }, [open]);

  const signIn = () => {
    setStep("oauth");
    void connect().then((ok) => {
      if (ok) onClose();
    });
  };

  const proceed = (e: FormEvent) => {
    e.preventDefault();
    signIn();
  };

  const cancelSignIn = () => {
    cancelConnect();
    onClose();
  };

  return (
    <AddAccountSheet
      open={open}
      title="Add account"
      onClose={onClose}
      oauth={
        step === "oauth"
          ? {
              provider: provider.provider,
              link: authUrl,
              error: phase === "error" ? error : null,
              note: "Margin only sees the files it creates in a margin folder on your Drive, never the rest of it.",
              onReopen: openAuthUrl,
              onCopy: () => void copyAuthUrl(),
              onRetry: signIn,
              onCancel: cancelSignIn,
            }
          : null
      }
    >
      <form className="add-account" onSubmit={proceed}>
        <label className="field">
          <span className="field-label">Provider</span>
          <select value={providerId} data-autofocus="" onChange={(e) => setProviderId(e.target.value)}>
            {PROVIDERS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </label>
        <p className="settings-note">
          {provider.provider} asks you to sign in on its own page in your browser. On that page, tick the box that lets
          Margin see, edit, create and delete only the Drive files it uses.
        </p>
        <div className="settings-actions">
          <Button type="submit" variant="primary">
            Continue
          </Button>
        </div>
      </form>
    </AddAccountSheet>
  );
}

function AboutSection() {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    if (!isDesktop) return;
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);
  return (
    <SettingsSection title="About">
      <SettingRow label="Margin" note="A calm, offline writing studio for authors.">
        {version ? <span className="settings-version">Version {version}</span> : null}
      </SettingRow>
      {isDesktop ? (
        <SettingRow label="Updates">
          <Button onClick={() => checkForUpdates(false)}>Check for updates</Button>
        </SettingRow>
      ) : null}
    </SettingsSection>
  );
}
