export { useEscapeLayer } from "./escape";

export { COMPACT_QUERY, PHONE_QUERY, TOUCH_QUERY, useCompact, useMediaQuery, usePhone, useTouch } from "./useMedia";

export { notify, useToast } from "./useToast";
export type { ToastAction, ToastState } from "./useToast";

export { createOverlays } from "./overlays";
export type { OverlayState } from "./overlays";

export {
  ALL_THEMES,
  THEMES,
  applyTheme,
  chosen,
  createThemeStore,
  labelOf,
  resolveTheme,
  schemeOf,
  storedPreference,
  syncWindowScheme,
  systemScheme,
  themeBootScript,
  themeKeys,
} from "./theme";
export type { BootConfig, Preference, Scheme, Theme, ThemeChoice, ThemeConfig, ThemeInfo, ThemeState } from "./theme";

export { isDesktop, isMacDesktop, isTauri } from "../ipc";
