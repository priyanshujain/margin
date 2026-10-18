import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  THEMES,
  chosen,
  createThemeStore,
  resolveTheme,
  storedPreference,
  themeBootScript,
  type Preference,
} from "./theme";

const read = (path: string): string => readFileSync(new URL(path, import.meta.url), "utf8");

interface Sheets {
  themes: Map<string, Set<string>>;
  schemes: Map<string, string>;
}

function declarations(): Sheets {
  const themes = new Map<string, Set<string>>();
  const schemes = new Map<string, string>();
  const css = read("../tokens.css").replace(/\/\*[\s\S]*?\*\//g, "");
  for (const rule of css.matchAll(/([^{}]+)\{([^}]*)\}/g)) {
    const selector = (rule[1].split(";").pop() ?? "").trim();
    const body = rule[2];
    const names = [...body.matchAll(/(--[a-z0-9-]+)\s*:/g)].map((m) => m[1]);
    const scheme = body.match(/color-scheme\s*:\s*([a-z]+)/)?.[1];
    for (const id of [...selector.matchAll(/:root\[data-theme="([a-z-]+)"\]/g)].map((m) => m[1])) {
      const set = themes.get(id) ?? new Set<string>();
      for (const name of names) set.add(name);
      themes.set(id, set);
      if (scheme) schemes.set(id, scheme);
    }
  }
  return { themes, schemes };
}

const fakeStorage = (entries: Record<string, string>) => {
  const store = new Map(Object.entries(entries));
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => store.set(k, v),
    },
  });
  return store;
};

describe("the palettes", () => {
  const { themes, schemes } = declarations();
  const full = new Set([...themes.values()].flatMap((set) => [...set]));

  it("declare the same variables as each other", () => {
    for (const info of THEMES) {
      const declared = themes.get(info.id) ?? new Set<string>();
      const missing = [...full].filter((name) => !declared.has(name));
      expect({ theme: info.id, missing }).toEqual({ theme: info.id, missing: [] });
    }
  });

  it("are exactly the ones the table names", () => {
    expect([...themes.keys()].sort()).toEqual(THEMES.map((info) => info.id).sort());
  });

  it("tell the webview which way round they are", () => {
    for (const info of THEMES) {
      expect([info.id, schemes.get(info.id)]).toEqual([info.id, info.scheme]);
    }
  });
});

describe("the boot script", () => {
  const settings = (script: string) => JSON.parse(script.slice(script.lastIndexOf("})(") + 3, -2));

  it("knows every offered theme split by scheme, and only those", () => {
    const all = settings(themeBootScript({ prefix: "app" }));
    for (const scheme of ["light", "dark"] as const) {
      const expected = THEMES.filter((info) => info.scheme === scheme).map((info) => info.id);
      expect([...all[scheme]].sort()).toEqual([...expected].sort());
    }
    const two = settings(themeBootScript({ prefix: "marginmail", palettes: ["light", "dark"] }));
    expect(two.light).toEqual(["light"]);
    expect(two.dark).toEqual(["dark"]);
    expect(two.keys.choice).toBe("marginmail-theme");
  });

  it("parses as a script", () => {
    expect(() => new Function(themeBootScript({ prefix: "app" }))).not.toThrow();
  });
});

describe("a stored preference", () => {
  it("survives a theme that no longer exists", () => {
    fakeStorage({
      "margindocs-theme": "solarized",
      "margindocs-theme-light": "sepia",
      "margindocs-theme-dark": "retired",
    });
    expect(storedPreference({ prefix: "margindocs" })).toEqual({
      choice: "system",
      light: "sepia",
      dark: "dark",
    });
  });

  it("keeps the one the two value releases wrote", () => {
    fakeStorage({ "marginmail-theme": "dark" });
    expect(resolveTheme(storedPreference({ prefix: "marginmail", palettes: ["light", "dark"] }))).toBe(
      "dark",
    );
  });

  it("ignores a palette the app does not offer", () => {
    fakeStorage({ "marginmail-theme": "midnight", "marginmail-theme-dark": "midnight" });
    expect(storedPreference({ prefix: "marginmail", palettes: ["light", "dark"] })).toEqual({
      choice: "system",
      light: "light",
      dark: "dark",
    });
  });

  it("remembers the half it is replacing", () => {
    const start: Preference = { choice: "system", light: "light", dark: "dark" };
    const midnight = chosen(start, "midnight");
    expect(midnight).toEqual({ choice: "midnight", light: "light", dark: "midnight" });
    expect(chosen(midnight, "mist")).toEqual({ choice: "mist", light: "mist", dark: "midnight" });
  });
});

describe("the theme toggle", () => {
  it("flips between the last light and the last dark palette chosen", () => {
    const store = fakeStorage({});
    const useTheme = createThemeStore({ prefix: "test" });
    const { select, toggle } = useTheme.getState();

    select("sepia");
    select("midnight");
    expect(useTheme.getState().theme).toBe("midnight");
    expect(store.get("test-theme")).toBe("midnight");

    toggle();
    expect(useTheme.getState().theme).toBe("sepia");

    toggle();
    expect(useTheme.getState().theme).toBe("midnight");
  });

  it("refuses a palette the app does not offer", () => {
    fakeStorage({});
    const useTheme = createThemeStore({ prefix: "test", palettes: ["light", "dark"] });
    useTheme.getState().select("midnight");
    expect(useTheme.getState().choice).toBe("system");
  });
});
