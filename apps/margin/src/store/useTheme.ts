import { createThemeStore } from "@margin/core/hooks";
import { THEME_PALETTES } from "./themePalettes";

export const useTheme = createThemeStore({ prefix: "margin", palettes: THEME_PALETTES });
