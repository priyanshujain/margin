import { createThemeStore } from "@margin/core/hooks";
import { THEME_PALETTES } from "./themePalettes";

export const useTheme = createThemeStore({ prefix: "margindocs", palettes: THEME_PALETTES });
