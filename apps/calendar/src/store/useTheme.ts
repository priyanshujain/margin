import { createThemeStore } from "@margin/core/hooks";
import { THEME_PALETTES } from "./themePalettes";

/** The storage key stays `margincal-theme`, so a choice made before margin-core still holds. */
export const useTheme = createThemeStore({ prefix: "margincal", palettes: THEME_PALETTES });
