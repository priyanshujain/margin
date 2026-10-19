// Cmd+, and the Settings row on the app menu. Four sections, and deliberately not a fifth: how the
// app looks, which version is running and whether there is a newer one, the two proofing checkers,
// and what this build is. Everything else this app can be told is already one click from the thing
// it changes, the editor width in the title bar most of all, and a second place to find a setting
// is worse than a longer walk to its only one.
//
// It takes the whole window rather than floating in the middle of it. A panel that has to hold a
// picker of seven palettes, each of them a picture, is not a panel any more, and a settings screen
// is the one surface in this app nobody is reading a document behind: dimming the page to look at
// it was never buying anything.
//
// A checker the machine does not have leaves its row on screen and turns it off. The store's own
// comment argues for taking a missing checker off the screen entirely, and that is right for
// underlines and for the correction menu: an underline that is absent explains itself. A settings
// screen is where somebody goes to look for a setting, and a row that is simply not there reads as
// a missing feature rather than as a missing checker, so the row stays and says which it is.

import { useEffect, useState, type ReactNode } from "react";
import { useKeyContext } from "@margin/core/keys";
import { SettingRow, SettingsFact, SettingsSection, SettingsShell } from "@margin/core/settings";
import { Button, Icon, Toggle } from "@margin/core/ui";
import { THEMES, labelOf, type Theme } from "@margin/core/hooks";
import { onCommand } from "../keys/commands";
import { useProofing } from "../store/useProofing";
import { useTheme } from "../store/useTheme";
import { useUpdate } from "../store/useUpdate";
import { appVersion, checkForUpdates } from "../update";

const BUILD = import.meta.env.DEV ? "development build" : "release build";

type Section = "appearance" | "updates" | "proofing" | "about";

const SECTIONS: readonly { id: Section; label: string }[] = [
  { id: "appearance", label: "Appearance" },
  { id: "updates", label: "Updates" },
  { id: "proofing", label: "Proofing" },
  { id: "about", label: "About" },
];

function checkedLabel(at: number | null): string {
  if (at === null) return "Not checked yet";
  const seconds = Math.max(0, Math.round((Date.now() - at) / 1000));
  if (seconds < 90) return "Checked just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `Checked ${minutes} minutes ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return hours === 1 ? "Checked an hour ago" : `Checked ${hours} hours ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return days === 1 ? "Checked yesterday" : `Checked ${days} days ago`;
  return `Checked on ${new Date(at).toLocaleDateString()}`;
}

function Swatch({ theme }: { theme: Theme }) {
  return (
    <span className="theme-swatch" data-theme={theme}>
      <span className="theme-swatch-rail" />
      <span className="theme-swatch-page">
        <span className="theme-swatch-line" />
        <span className="theme-swatch-line" />
        <span className="theme-swatch-dot" />
      </span>
    </span>
  );
}

interface ThemeCardProps {
  active: boolean;
  label: string;
  preview: ReactNode;
  onSelect: () => void;
}

function ThemeCard({ active, label, preview, onSelect }: ThemeCardProps) {
  return (
    <button
      type="button"
      className="theme-card"
      data-active={active}
      aria-pressed={active}
      onClick={onSelect}
    >
      <span className="theme-preview">
        {preview}
        {active && (
          <span className="theme-tick" aria-hidden="true">
            <Icon d="M5 12.5l4.5 4.5L19 7" size={13} />
          </span>
        )}
      </span>
      <span className="theme-name">{label}</span>
    </button>
  );
}

/**
 * The palettes, applied on the click and never behind a save button.
 *
 * A colour is judged by looking at it, so the window behind this one changes while the picker is
 * still open. There is nothing to confirm and nothing to undo beyond clicking a different tile,
 * which is also why the tiles are the whole control: a list of names would make somebody apply each
 * one to find out what it was.
 */
function ThemePicker() {
  const choice = useTheme((s) => s.choice);
  const light = useTheme((s) => s.light);
  const dark = useTheme((s) => s.dark);
  const select = useTheme((s) => s.select);

  return (
    <div className="theme-grid">
      <ThemeCard
        active={choice === "system"}
        label="Match system"
        preview={
          <span className="theme-pair">
            <Swatch theme={light} />
            <Swatch theme={dark} />
          </span>
        }
        onSelect={() => select("system")}
      />
      {THEMES.map((info) => (
        <ThemeCard
          key={info.id}
          active={choice === info.id}
          label={info.label}
          preview={<Swatch theme={info.id} />}
          onSelect={() => select(info.id)}
        />
      ))}
    </div>
  );
}

export function useSettingsOpen(): [boolean, (open: boolean) => void] {
  const [open, setOpen] = useState(false);
  useEffect(() => onCommand("settings", () => setOpen((v) => !v)), []);
  return [open, setOpen];
}

export function Settings({ onClose }: { onClose: () => void }) {
  const [section, setSection] = useState<Section>("appearance");
  const [version, setVersion] = useState<string | null>(null);

  const phase = useUpdate((s) => s.phase);
  const lastChecked = useUpdate((s) => s.lastChecked);
  const automatic = useUpdate((s) => s.automatic);
  const setAutomatic = useUpdate((s) => s.setAutomatic);

  const light = useTheme((s) => s.light);
  const dark = useTheme((s) => s.dark);

  const spelling = useProofing((s) => s.enabled);
  const setSpelling = useProofing((s) => s.setEnabled);
  const spellingAvailability = useProofing((s) => s.availability);
  const grammar = useProofing((s) => s.grammar);
  const setGrammar = useProofing((s) => s.setGrammar);
  const grammarAvailability = useProofing((s) => s.grammarAvailability);
  const ensureAvailable = useProofing((s) => s.ensureAvailable);

  useKeyContext("overlay");

  useEffect(() => {
    ensureAvailable();
    let live = true;
    void appVersion().then((v) => {
      if (live) setVersion(v);
    });
    return () => {
      live = false;
    };
  }, [ensureAvailable]);

  const checking = phase === "checking";
  const spellingMissing = spellingAvailability === "missing";
  const grammarMissing = grammarAvailability === "missing";

  return (
    <div className="settings-stage">
      <SettingsShell
        sections={SECTIONS}
        active={section}
        onSelect={(id) => setSection(id as Section)}
        onClose={onClose}
        back={{ label: "Back to Docs", onBack: onClose }}
      >
        {section === "appearance" && (
          <SettingsSection
            title="Appearance"
            note={`Match system follows the operating system between ${labelOf(light)} and ${labelOf(dark)}, which are the last of each you chose.`}
          >
            <ThemePicker />
          </SettingsSection>
        )}

        {section === "updates" && (
          <SettingsSection title="Updates">
            <SettingRow label="Software update" note={checkedLabel(lastChecked)}>
              <Button disabled={checking} onClick={() => void checkForUpdates()}>
                {checking ? "Checking…" : "Check Now"}
              </Button>
            </SettingRow>
            <Toggle
              label="Check automatically"
              note="Once a day, in the background, on launch."
              checked={automatic}
              onChange={setAutomatic}
            />
          </SettingsSection>
        )}

        {section === "proofing" && (
          <SettingsSection title="Proofing">
            <Toggle
              label="Check spelling while typing"
              note={spellingMissing ? "This machine has no spell checker." : undefined}
              checked={spelling && !spellingMissing}
              disabled={spellingMissing}
              onChange={setSpelling}
            />
            <Toggle
              label="Check grammar while typing"
              note={grammarMissing ? "This build has no grammar checker in it." : undefined}
              checked={grammar && !grammarMissing}
              disabled={grammarMissing}
              onChange={setGrammar}
            />
          </SettingsSection>
        )}

        {section === "about" && (
          <SettingsSection title="About">
            <SettingsFact label="App">Margin Docs</SettingsFact>
            <SettingsFact label="Version">
              {version === null ? BUILD : `${version}, ${BUILD}`}
            </SettingsFact>
          </SettingsSection>
        )}
      </SettingsShell>
    </div>
  );
}
