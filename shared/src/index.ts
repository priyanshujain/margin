// Everything both apps draw from. Two entry points as well as this one, `margin-shared/fonts` and
// `margin-shared/icons`, so a module that only wants the glyphs does not pull in the catalogue.

export * from "./fonts";
export * as icons from "./icons";
