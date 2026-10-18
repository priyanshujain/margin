import { createThemeStore } from "@margin/core/hooks";
import { THEME_PALETTES } from "./themePalettes";

/** The storage keys stay `marginmail-theme*`, so a choice made before margin-core still holds. */
export const useTheme = createThemeStore({ prefix: "marginmail", palettes: THEME_PALETTES });
