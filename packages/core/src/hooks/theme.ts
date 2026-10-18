import { getCurrentWindow } from "@tauri-apps/api/window";
import { create } from "zustand";
import { isDesktop } from "../ipc";

export type Scheme = "light" | "dark";

export type Theme = "light" | "sepia" | "mist" | "contrast" | "dark" | "graphite" | "midnight";

export type ThemeChoice = Theme | "system";

export interface ThemeInfo {
  readonly id: Theme;
  readonly label: string;
  readonly scheme: Scheme;
}

export const THEMES: readonly ThemeInfo[] = [
  { id: "light", label: "Warm Paper", scheme: "light" },
  { id: "sepia", label: "Sepia", scheme: "light" },
  { id: "mist", label: "Mist", scheme: "light" },
  { id: "contrast", label: "High Contrast", scheme: "light" },
  { id: "dark", label: "Warm Night", scheme: "dark" },
  { id: "graphite", label: "Graphite", scheme: "dark" },
  { id: "midnight", label: "Midnight", scheme: "dark" },
];

export const ALL_THEMES: readonly Theme[] = THEMES.map((info) => info.id);

export interface Preference {
  readonly choice: ThemeChoice;
  readonly light: Theme;
  readonly dark: Theme;
}

export interface ThemeConfig {
  prefix: string;
  palettes?: readonly Theme[];
}

export function schemeOf(id: Theme): Scheme {
  return THEMES.find((info) => info.id === id)?.scheme ?? "light";
}

export function labelOf(id: Theme): string {
  return THEMES.find((info) => info.id === id)?.label ?? id;
}

export function systemScheme(): Scheme {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function resolveTheme(preference: Preference): Theme {
  if (preference.choice !== "system") return preference.choice;
  return systemScheme() === "dark" ? preference.dark : preference.light;
}

export function chosen(preference: Preference, choice: ThemeChoice): Preference {
  if (choice === "system") return { ...preference, choice };
  const scheme = schemeOf(choice);
  return {
    choice,
    light: scheme === "light" ? choice : preference.light,
    dark: scheme === "dark" ? choice : preference.dark,
  };
}

function saved(key: string): string | null {
  if (typeof localStorage === "undefined") return null;
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function store(key: string, value: string): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(key, value);
  } catch {
    return;
  }
}

function offered(palettes: readonly Theme[] | undefined): readonly Theme[] {
  const list = (palettes ?? ALL_THEMES).filter((id) => ALL_THEMES.includes(id));
  return list.length > 0 ? list : ALL_THEMES;
}

function fallback(palettes: readonly Theme[], scheme: Scheme): Theme {
  if (palettes.includes(scheme)) return scheme;
  return palettes.find((id) => schemeOf(id) === scheme) ?? palettes[0];
}

export function themeKeys(prefix: string) {
  return {
    choice: `${prefix}-theme`,
    light: `${prefix}-theme-light`,
    dark: `${prefix}-theme-dark`,
  };
}

export function storedPreference(config: ThemeConfig): Preference {
  const palettes = offered(config.palettes);
  const keys = themeKeys(config.prefix);
  const isOffered = (value: string | null): value is Theme =>
    value !== null && palettes.includes(value as Theme);
  const half = (key: string, scheme: Scheme): Theme => {
    const value = saved(key);
    return isOffered(value) && schemeOf(value) === scheme ? value : fallback(palettes, scheme);
  };
  const choice = saved(keys.choice);
  return {
    choice: choice === "system" || isOffered(choice) ? choice : "system",
    light: half(keys.light, "light"),
    dark: half(keys.dark, "dark"),
  };
}

export function applyTheme(theme: Theme): void {
  if (typeof document === "undefined") return;
  document.documentElement.setAttribute("data-theme", theme);
}

export function syncWindowScheme(scheme: Scheme | null): void {
  if (isDesktop) getCurrentWindow().setTheme(scheme).catch(() => {});
}

function persist(prefix: string, preference: Preference): void {
  const keys = themeKeys(prefix);
  store(keys.choice, preference.choice);
  store(keys.light, preference.light);
  store(keys.dark, preference.dark);
}

function watchSystemScheme(onChange: () => void): void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", onChange);
}

export interface ThemeState {
  palettes: readonly Theme[];
  choice: ThemeChoice;
  theme: Theme;
  light: Theme;
  dark: Theme;
  select: (choice: ThemeChoice) => void;
  toggle: () => void;
}

export function createThemeStore(config: ThemeConfig) {
  const palettes = offered(config.palettes);
  const start = storedPreference(config);
  syncWindowScheme(start.choice === "system" ? null : schemeOf(resolveTheme(start)));

  const useTheme = create<ThemeState>((set, get) => ({
    palettes,
    choice: start.choice,
    theme: resolveTheme(start),
    light: start.light,
    dark: start.dark,

    select: (choice) => {
      if (choice !== "system" && !palettes.includes(choice)) return;
      const { choice: was, light, dark } = get();
      const next = chosen({ choice: was, light, dark }, choice);
      const theme = resolveTheme(next);
      applyTheme(theme);
      syncWindowScheme(next.choice === "system" ? null : schemeOf(theme));
      persist(config.prefix, next);
      set({ choice: next.choice, light: next.light, dark: next.dark, theme });
    },

    toggle: () => {
      const { theme, light, dark } = get();
      get().select(schemeOf(theme) === "dark" ? light : dark);
    },
  }));

  watchSystemScheme(() => {
    const { choice, select } = useTheme.getState();
    if (choice === "system") select("system");
  });

  return useTheme;
}

export interface BootConfig extends ThemeConfig {
  phoneQuery?: string;
  touchQuery?: string;
}

export function themeBootScript(config: BootConfig): string {
  const palettes = offered(config.palettes);
  const keys = themeKeys(config.prefix);
  const light = palettes.filter((id) => schemeOf(id) === "light");
  const dark = palettes.filter((id) => schemeOf(id) === "dark");
  const settings = {
    keys,
    light,
    dark,
    fallback: { light: fallback(palettes, "light"), dark: fallback(palettes, "dark") },
    phone: config.phoneQuery ?? "(max-width: 640px)",
    touch: config.touchQuery ?? "(pointer: coarse)",
  };
  return `(function (c) {
  var root = document.documentElement;
  function saved(key) {
    try {
      return localStorage.getItem(key);
    } catch (e) {
      return null;
    }
  }
  var night = matchMedia("(prefers-color-scheme: dark)").matches;
  var choice = saved(c.keys.choice);
  var theme;
  if (c.light.indexOf(choice) !== -1 || c.dark.indexOf(choice) !== -1) {
    theme = choice;
  } else {
    var pool = night ? c.dark : c.light;
    var pick = saved(night ? c.keys.dark : c.keys.light);
    theme = pool.indexOf(pick) !== -1 ? pick : night ? c.fallback.dark : c.fallback.light;
  }
  root.setAttribute("data-theme", theme);
  if (matchMedia(c.phone).matches) root.setAttribute("data-phone", "");
  if (matchMedia(c.touch).matches) root.setAttribute("data-touch", "");
})(${JSON.stringify(settings)});`;
}
