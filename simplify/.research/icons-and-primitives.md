# Icons and the smallest UI primitives

Audit of `margin` (`/Users/pj/Workspace/projects/python/margin`), `margin-calendar`
(`/Users/pj/Workspace/projects/python/margin-caledar`), `margin-docs`
(`/Users/pj/Workspace/projects/rust/margin-editor`) and `margin-mail`
(`/Users/pj/Workspace/projects/rust/margin-mail`) against the partial shared package at
`/Users/pj/Workspace/projects/python/margin/shared`.

## The shared package as it stands

`shared/src/icons.ts` exports twelve paths plus `SUN_DISC`. Its header comment says the two apps
kept drifting, that a path is a design decision, and that `Icon` is deliberately not shared because
sharing it "would make this package depend on React for twenty four lines, and a component is where
an app is entitled to differ."

Two of those three claims no longer hold.

The React argument is wrong on the mechanics. `shared/package.json` has no `dependencies` block at
all, no build step, and every app resolves the TypeScript source through its own bundler. All four
apps are on `react: ^19.1.0`. A `peerDependencies` entry costs zero bytes and installs nothing;
it is a version assertion, not a dependency.

"An app is entitled to differ" is contradicted by the code. Three of the four `Icon.tsx` files are
byte identical (md5 `0ec1a568818f20ed8eed8ad46fbaa2b1`):
`/Users/pj/Workspace/projects/python/margin/src/components/Icon.tsx`,
`/Users/pj/Workspace/projects/python/margin-caledar/src/components/Icon.tsx`,
`/Users/pj/Workspace/projects/rust/margin-editor/src/components/Icon.tsx`. In two years nobody has
exercised the entitlement.

Also worth noting: `margin-calendar` does not consume `margin-shared` at all. There is no
`margin-shared` line in `/Users/pj/Workspace/projects/python/margin-caledar/package.json`, and
`src/styles/tokens.css` is a hand copy of the shared file's values. Everything below that looks
like calendar drifting away from the family traces back to this one fact.

## The Icon component, line by line

All four render the same SVG: `viewBox="0 0 24 24"`, `fill="none"`, `stroke="currentColor"`,
`strokeWidth="1.6"`, `strokeLinecap="round"`, `strokeLinejoin="round"`, `size = 16` default,
`{children ?? <path d={d} />}`. Props are `d?: string`, `size?: number`, `children?: ReactNode`.

`margin-mail`'s at `/Users/pj/Workspace/projects/rust/margin-mail/src/ui/Icon.tsx:12` is the only
one that differs, in four ways, all of them improvements:

- `export interface IconProps` rather than a private `interface` (line 4).
- `className="icon"` (line 15), which is what lets CSS reach the element.
- `aria-hidden="true"` (line 24). The other three emit an unlabelled SVG into the accessibility
  tree at every one of their 142 combined call sites.
- `import "./Icon.css"` (line 2), whose entire contents are `.icon { flex: none; }`
  (`Icon.css:2-4`).

There is no alignment handling in any of the four components. No `display`, no `vertical-align`,
no `shape-rendering`, no `vector-effect`, no transform.

## Alignment: the thing that keeps being fixed four times

Across all four repos there are **zero** occurrences of `shape-rendering`, `vector-effect`,
`crispEdges`, `geometricPrecision`, or a `translate(0.5 0.5)` style half pixel offset. The
alignment problem is not sub-pixel rasterisation. It is the two ordinary CSS facts about an inline
SVG: it sits on the text baseline, and it is a flex item that will shrink.

Five different fixes exist for those two facts, and only one app fixes them centrally.

`margin-mail` fixes both once:

- `/Users/pj/Workspace/projects/rust/margin-mail/src/styles/app.css:88` `svg { display: block; }`
  This is the only global SVG rule in the suite. The other three apps have no `svg` selector at
  document level at all.
- `/Users/pj/Workspace/projects/rust/margin-mail/src/ui/Icon.css:2` `.icon { flex: none; }`

The other three patch it per site:

- `/Users/pj/Workspace/projects/python/margin-caledar/src/styles/overlays.css:36`
  `.panel-note[data-icon] svg { flex: none; transform: translateY(2px); }`
- `/Users/pj/Workspace/projects/python/margin-caledar/src/styles/details.css:114`
  `.details-row[data-block] > svg { margin-top: 2px; }`
- `/Users/pj/Workspace/projects/rust/margin-editor/src/styles/tree.css:473`
  `.start-row svg { align-self: center; color: var(--ink-faint); }`

A `translateY(2px)` and a `margin-top: 2px` in the same repo, for the same symptom, four files
apart. Neither is wrong; both exist because the baseline was never dealt with at the root.

The residual case is real and survives the global fix: an icon inside an `align-items: baseline`
row still needs `align-self: center`. `margin-mail` hits it too, at
`/Users/pj/Workspace/projects/rust/margin-mail/src/ui/Row.css:127` (`.row-mark { flex: none;
display: inline-flex; align-self: center; }`), which is the same declaration as margin-docs'
`tree.css:473`. There are 20 `align-items: baseline` rules across the four apps, so this is a
recurring shape, not an exception.

Icon size is not a shared decision and probably should not become one. `margin-mail` never uses the
16px default (zero bare `<Icon d=... />`, nine distinct explicit sizes from 10 to 20). The other
three lean on the default heavily: 33 bare call sites in margin-docs, 14 in calendar, 12 in margin.

## Glyph inventory

151 path definitions across the suite, 132 distinct strings, 13 of which appear in more than one
app. The shared set covers 13. Per app, unique path strings: shared 13, margin 30, calendar 31,
margin-docs 47, margin-mail 30.

Only `margin-mail` keeps its glyphs in a module (`src/ui/icons.ts`, 30 named constants, five
re-exported from `margin-shared/icons` at line 14). `margin-docs` names its toolbar and titlebar
glyphs as module constants but writes six more inline. `margin` and `margin-calendar` are almost
entirely inline `d="M..."` in JSX.

Shared-set uptake is thin: `margin` uses eleven of the twelve; `margin-docs` uses seven
(`SIDEBAR`, `SEARCH`, `SPELLING`, `GRAMMAR`, `EXPORT`, `MORE`, `CHECK`, `WIDTH`); `margin-mail`
re-exports five; `margin-calendar` uses none.

### Same concept, different path

The important cases, with the exact strings.

**SEARCH.** Shared `icons.ts:21` is `M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4`. Calendar
`components/Header.tsx:22` is `M11 19a8 8 0 100-16 8 8 0 000 16zM21 21l-4.35-4.35`. Different lens
radius (7 vs 8) and a different handle. This is the exact divergence the shared package's header
comment says it exists to prevent, still present because calendar never joined.

**MORE.** Shared `icons.ts:57` is three dots, `M5 12h.01M12 12h.01M19 12h.01`. Calendar
`components/PhoneBar.tsx:29` uses the same name for a hamburger, `M4 7h16M4 12h16M4 17h16`. A
straight name collision on two unrelated glyphs.

**SUN.** Shared splits it: `SUN_RAYS` (`icons.ts:53`) with a `SUN_DISC` circle at `r: 4`, rays
starting at `M12 2v2`. Calendar `Header.tsx:24` is one path with an `r=5` disc and rays at
`M12 1v2M12 21v2M4.2 4.2...`. Different construction and different geometry.

**HEADING.** `margin/src/editor/FloatingToolbar.tsx:126` is `M5 5v14M5 12h8M13 5v14`.
`margin-editor/src/editor/Toolbar.tsx:184` is `M7 5v14M7 12h10M17 5v14`, with a comment at line 182
that says exactly why: "The H used to run from x=5 to x=13 in a 24 unit box, so it sat left of
centre in a round button that every other glyph here is centred in." One app fixed the optical
centring; the other still has the bug. This is the "fix the alignment separately in each app"
complaint, at the glyph level, with the fix already written down in one repo.

**BULLET LIST.** `margin/src/editor/FloatingToolbar.tsx:128` puts the bullets at x=3.5:
`M8 6h12M8 12h12M8 18h12M3.5 6h.01M3.5 12h.01M3.5 18h.01`.
`margin-editor/src/editor/Toolbar.tsx:185` puts them at x=4: `...M4 6h.01M4 12h.01M4 18h.01`. A half
unit apart on otherwise identical rules.

**TRASH.** `margin/src/components/RowMenu.tsx:153` and `margin-editor/src/components/Sidebar.tsx:46`
agree: `M5 7h14M10 7V5h4v2M7 7l1 13h8l1-13M10 11v6M14 11v6`. `margin-mail/src/ui/icons.ts:41` is a
different drawing: `M4 7h16M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2M6 7l1 13a1 1 0 0 0 1 1h8a1 1 0 0
0 1-1l1-13M10 11v6M14 11v6`. Wider (4 to 20 rather than 5 to 19) and with rounded corners.

**LINK.** margin `FloatingToolbar.tsx:153` and margin-docs `Toolbar.tsx:192` agree on `l2-2`.
Calendar `EventDetails.tsx:46` and `EventEditor.tsx:42` use `l3-3`, a longer link arm.

**REFRESH.** margin `BackupSettings.tsx:67` is `M21 12a9 9 0 1 1-2.6-6.4M21 4v5h-5`. Calendar
`Header.tsx:23` is `M21 12a9 9 0 11-3-6.7M21 3v6h-6`. Different arc endpoint and a different arrow.

**CHECK.** Shared `icons.ts:61` is `M20 6L9 17l-5-5`, used by margin-docs `WidthMenu.tsx:177` at
size 14. margin-docs also draws its own at `components/Settings.tsx:131`,
`d="M5 12.5l4.5 4.5L19 7"` at size 13. One app, two ticks.

**BOLD and ITALIC.** margin renders letterforms, `<b>B</b>` and `<i>I</i>`
(`FloatingToolbar.tsx:123-124`). margin-docs draws paths, `BOLD_D` and `ITALIC_D`
(`Toolbar.tsx:177-178`). Same toolbar, same button, two different answers to what a bold button is.

### Same drawing, different spelling

These render identically and are only string-level drift, but they are what makes a `grep` for
duplication useless.

- **CLOSE**, five spellings: shared `M6 6l12 12M18 6L6 18`; `M18 6L6 18M6 6l12 12` in calendar
  `EventDetails.tsx:38`, margin-docs `Recents.tsx:23`, `Sidebar.tsx:47`, `Toolbar.tsx:194`,
  `Settings.tsx:253`, margin `FindBar.tsx:248`; `M18 6 6 18M6 6l12 12` in calendar
  `overlayShell.tsx:14`.
- **MOON**: shared `A9 9 0 1 1 11.2 3` versus calendar `A9 9 0 1111.2 3`. Packed arc flags, same
  curve.
- **CLOCK**: calendar `EventDetails.tsx:44` `M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0M12 7v5l3 2`
  versus mail `icons.ts:46` `M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7v5l3 2`.
- **CHEVRON_RIGHT**: calendar `M9 18l6-6-6-6` versus mail `M9 6l6 6-6 6`. Drawn from opposite ends.
- **DUPLICATE**: margin `RowMenu.tsx:142` `M9 9h11v11h-11z M6 15V5h9` versus margin-docs
  `Sidebar.tsx:42` `M9 9h11v11H9z M6 15V5h9`.

### Exact duplicates that are not in the shared set

`PLUS` (`M12 5v14M5 12h14`) is defined independently in all four apps. `CHEVRON_UP`/`CHEVRON_DOWN`
(`M6 15l6-6 6 6` / `M6 9l6 6 6-6`) three times. Vertical dots (`M12 5h.01M12 12h.01M12 19h.01`),
`MINUS`, `HR`, `IMAGE`, `BLOCKQUOTE` twice each, always margin and margin-docs.

## The icon button: eleven rules for one control

All four share a byte-identical `button` reset (`margin app.css:34`, `calendar app.css:63`,
`docs app.css:34`, `mail app.css:70`, the last adding `font-size: inherit`). On top of it:

| App | Class | Size | Radius | Idle | Hover |
| --- | --- | --- | --- | --- | --- |
| margin | `.icon-btn` (`app.css:103`) | 30 | `--r-sm` | `--ink-soft` | `--accent-wash` |
| margin | `.find-btn` (`app.css:2255`) | 26 | `--r-sm` | `--ink-soft` | `--accent-wash` |
| margin | `.row-menu-btn` (`app.css:393`) | 22 | `--r-sm` | `--ink-faint` | `--accent-wash` |
| calendar | `.icon-button` (`app.css:145`) | 28 | `--r-sm` | `--ink-soft` | `--accent-wash` |
| calendar | `.details-close` (`details.css:269`) | 26 | `--r-sm` | `--ink-faint` | `--accent-wash` |
| docs | `.icon-button` (`app.css:138`) | 28 | `--r-sm` | `--ink-soft` | `--accent-wash` |
| docs | `.find-btn` (`tree.css:585`) | 26 | `--r-sm` | `--ink-soft` | `--accent-wash` |
| docs | `.start-forget` (`tree.css:450`) | 26 | `--r-sm` | `--ink-faint` | `--accent-wash` |
| docs | `.row-menu-btn` (`app.css:363`) | 22 | `--r-sm` | `--ink-faint` | `--accent-wash` |
| docs | `.tree-twisty` (`tree.css:189`) | 16 | `--r-sm` | `--ink-faint` | `--accent-wash` |
| mail | `.button[data-icon-only][data-variant="ghost"]` | 28 via `aspect-ratio: 1` | `--r-sm` | `--ink-soft` | `--accent-wash` |

Every one of them is `display: grid; place-items: center` (except `.start-forget`, which spells it
out as flex, and mail, which is inline-flex) with the same radius token, the same hover wash and
one of two colour tokens. `margin`'s `.row-menu-btn` and margin-docs' `.row-menu-btn` are the same
block copied verbatim into two repos. 26px appears four times across three apps.

The name is the only thing that reliably differs: `.icon-btn` in margin, `.icon-button` in the
other two.

The "on" state is where they genuinely disagree. margin `app.css:118` and margin-docs
`app.css:156` are the same three declarations (`color: var(--accent); background:
var(--accent-wash); box-shadow: inset 0 0 0 1px var(--line-strong)`) under two attribute names,
`data-on="true"` and `data-active="true"`. Calendar `app.css:160` drops the ring and uses
`--ink`. Mail `Button.css:69` makes `[data-active]` identical to `:hover`, so an open panel's
button and a hovered button are the same picture. Four apps, four answers, two of them pixel
identical under different attribute names. Attribute usage is mixed inside every app too: margin
21 `data-on` and 2 `data-active`, calendar 6 and 4, docs 12 and 9, mail 9 and 8.

There is no icon-button component anywhere except `margin-mail`. 54 call sites across the three
older apps hand-write `<button className="icon-btn|icon-button" title=... onClick=...><Icon
d={...} /></button>`: margin 18, calendar 16, margin-docs 20. `margin-mail` has 115 `<Button>`
usages and 16 `iconOnly` ones, and `Button.tsx:57` supplies the accessible name automatically:
`aria-label={label ?? (iconOnly ? title : undefined)}`.

The floating toolbar button is forked in the worst way. `margin app.css:893` `.tool` and
`margin-editor app.css:623` `.tool` are the same rule except one writes `border-radius: 999px` and
the other `border-radius: var(--r-pill)`. The same literal-versus-token split repeats on
`.editor-toolbar` (`margin app.css:889` vs `docs app.css:619`). The JS helpers differ only in
arity: `margin/src/editor/FloatingToolbar.tsx:109` is a render-scoped arrow with four positional
params; `margin-editor/src/editor/Toolbar.tsx:156` is a module function with the same four plus
`disabled`. Both carry the identical `onMouseDown={(e) => e.preventDefault()}`.

## Focus, disabled, tooltips, badges, spinners, keycaps

**Focus rings** are the one thing all four already agree on, byte for byte:
`:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }` at
`margin app.css:43`, `calendar app.css:80`, `docs app.css:51`, `mail app.css:92`. Nobody ships a
polyfill or does keyboard-versus-mouse detection. The divergence is in the exceptions: 18 sites
across the suite write `outline: none` with no replacement, and only `margin-mail` invents a second
ring colour (`screens/settings.css:467`, `outline: 2px solid var(--accent-wash)` at 1px offset).
The round-control case is handled twice and missed once: calendar `create.css:445` and docs
`toolbar.css:157` both use a two-layer box-shadow (`0 0 0 2px var(--paper), 0 0 0 3px
var(--accent)` and `0 0 0 1.5px ...` respectively, radii disagree), while mail's 15px round swatch
has no focus rule and gets the square outline that calendar's comment at `create.css:435` warns
about. `/Users/pj/Workspace/projects/python/margin/src/focus.ts` is the only focus-trap module in
the family; the other three have none.

**Disabled** has no agreement at all: five opacity values across four apps. margin uses 0.4, 0.5
and 0.6 in one file; calendar uses 0.45; margin-docs uses 0.4, 0.45 and 0.5; margin-mail mostly
abandons opacity for `color: var(--ink-faint); background: var(--raised)`
(`ui/Button.css:84`), which is the same recipe calendar reached independently at
`overlays.css:85`. Only one site in the suite pairs `:disabled` with `pointer-events: none`
(`docs export-preview.css:75`).

**Tooltips** do not exist as a component in any app. All four use the native `title` attribute:
44, 40, 66 and 71 occurrences. margin is the outlier on labelling, 44 `title` against 5
`aria-label`, so most of its icon buttons are unnamed to a screen reader; the other three run
21/43/42. The text generator is forked: margin-docs `Titlebar.tsx:105` `shortcutTitle(id)` returns
a whole string, calendar `Header.tsx:33` `hint(command)` returns a leading-space suffix, and margin
hardcodes `title="Find (⌘F)"` (`EditorView.tsx:294`) and `"Link (⌘K)"`
(`FloatingToolbar.tsx:153`), which are not platform aware.

**Badges** share one recipe and disagree on every number: a wash-tinted micro chip at
`padding: 1px 5|6|9px; background: var(--accent-wash); color: var(--ink-faint); font-size:
var(--t-1)`, in calendar `details.css:93`, `overlays.css:487`, `agenda.css:90` and mail
`tour.css:127`, with the radius `--r-sm` in calendar and `--r-pill` in mail. Status dots come in
5, 6, 7, 8 and 9px, and `border-radius: 50%` and `var(--r-pill)` are both used within one repo
(`docs app.css:132` vs `toolbar.css:311`). margin and margin-docs share three copy-pasted classes
verbatim: `.dirty-dot`, `.preview-count`, `.find-count`. `font-variant-numeric: tabular-nums` on
counts is used by all four.

**Loading** is the deepest split, and it is a product decision rather than an oversight. margin
and margin-docs have rotating spinners (`margin app.css:1461` `.spinner`, plus a byte-identical
`backup-spin` duplicate of `spin` at `:2711`; `docs export-preview.css:139` `.preview-spinner`).
`margin-calendar` has no spinner, no skeleton and no loading keyframes at all; it expresses
pending state as `[data-busy]` and `[data-pending]` on the content itself. `margin-mail` bans
spinners in three separate comments and uses bars and skeletons instead. Reduced motion is handled
in three different ways: margin has no guard at all, docs slows the spinner from 0.7s to 2.4s, mail
disables outright in five places. Do not try to unify this; the four apps mean different things.

**Keycaps.** calendar `palette.css:104` `.key` and mail `ui/Key.css:1` + `[data-size="md"]` are the
same chip: bordered, `--raised`, `--r-sm`, `--t-1`, `min-width: 20px`, `padding: 2px 6px`,
`line-height: 1.4`. margin-docs `tree.css:744` `.key-cap` is a different chip, wash-filled with no
border at `--t-2` and weight 600. margin has no chip, one rule
(`app.css:2191` `.esc-hint kbd`). Only `margin-mail` has a `Key` component
(`ui/Key.tsx:15`), only mail puts a cap on ordinary buttons, and only mail hides caps on phones
(`Key.css:28`). Underneath, `keys/bindings.ts` in calendar, docs and mail declare identical `isMac`
and `PRIMARY_LABEL` lines and an identical eight-entry `NAMED` map, then implement `keyLabel()`
three different ways: calendar (`:110`) cannot express `⌘⇧F` at all, docs (`:243`) infers shift
from case, mail (`:579`) treats shift as a first-class modifier. margin has no bindings table.

## Titlebar and window chrome

All four are Tauri v2 with `"titleBarStyle": "Overlay"` and native traffic lights. Nobody draws
window controls, nobody sets `decorations`, `hiddenTitle`, `transparent` or `macOSPrivateApi`, and
nobody uses `startDragging` or `-webkit-app-region`; every drag region is the
`data-tauri-drag-region` attribute.

The `.titlebar` rule is the same nine declarations in all four
(`margin app.css:67`, `calendar app.css:102`, `docs app.css:76`, `mail header.css:7`):
`flex: none; position: relative; z-index: 45; height: var(--titlebar-h); display: grid;
grid-template-columns: 1fr auto 1fr; align-items: center; background: var(--shell); border-bottom:
1px solid var(--line)`. Differences: margin and calendar and mail set `user-select: none`, docs
does not; calendar folds `--safe-top` into the height and padding.

The lane for the traffic lights is 84px in all four and is reserved three different ways.
`margin app.css:74` hardcodes it in `padding: 0 14px 0 84px`, unconditionally, with no token and no
platform gate, so Linux and Windows get a dead 84px lane. Calendar (`app.css:126`) and mail
(`header.css:24`) put `padding-left: var(--traffic-pad)` on the row under `:root[data-traffic]`.
margin-docs puts it on the child instead, `:root[data-traffic] .titlebar .lead { margin-left:
calc(var(--traffic-pad) - 14px) }` (`app.css:97`), with a comment explaining that padding on the
row pushed the centred title 35px right of the middle. Calendar's view switcher and mail's
`<Segment>` are both in centre columns and are subject to exactly that offset.

Only margin-docs has native code. `/Users/pj/Workspace/projects/rust/margin-editor/src-tauri/src/titlebar.rs`
resizes the `NSTitlebarContainerView` on `Resized`, `Focused` and `ThemeChanged` so the lights
centre in a 46px row, with a `const TITLEBAR_H: f64 = 46.0` at line 79 that duplicates
`--titlebar-h: 46px` from `shared/css/tokens.css:24`. Calendar and mail instead set
`"trafficLightPosition": { "x": 9, "y": 25 }` in `tauri.conf.json` and never reapply. margin does
neither, so its lights sit at the macOS default, roughly 7px high in a 46px row, which is the
misalignment `titlebar.rs` was written to fix.

`--traffic-pad: 84px` is declared four times (`calendar tokens.css:11` and `:60`,
`docs tokens.css:11`, `mail mail.css:36`) and is not in `margin-shared`. So is
`--r-pill: 999px` and `--touch-h: 44px`, three copies each. margin declares none of them and
inlines the literals.

## What to share, and what not to

**Share, high confidence:**

1. `Icon` itself. Three byte-identical copies plus one strictly better fourth. Move
   `margin-mail`'s version (className, `aria-hidden`, exported props type) to
   `shared/src/Icon.tsx` with `react` as a peer dependency. The stated reason not to has no
   mechanical basis.
2. The two lines of alignment that go with it: `svg { display: block }` and `.icon { flex: none }`,
   as `shared/css/icon.css`. This is the fix that has been made five different ways in four repos
   and is the direct answer to "I keep fixing icon alignment separately."
3. The rest of the glyphs. Promote `PLUS`, `CHEVRON_UP/DOWN/LEFT/RIGHT`, vertical dots, `MINUS`,
   `HR`, `IMAGE`, `BLOCKQUOTE`, `TRASH`, `LINK`, `REFRESH`, `CLOCK`, `DOCUMENT`, `COPY`,
   `EXTERNAL`, `BOLD`, `ITALIC`, `HEADING`, `BULLET_LIST` into `shared/src/icons.ts`, picking the
   better drawing where they have drifted (margin-docs' `HEADING_D` and `BULLET_LIST_D`, the
   margin/margin-docs `TRASH` and `LINK`, the shared `SEARCH` and `MORE` and `SUN`). Then delete
   every inline `d="M..."` from JSX. This turns 151 definitions into roughly 60.
4. `--r-pill`, `--touch-h` and `--traffic-pad` into `shared/css/tokens.css`. Three copies each of a
   single number, and in `--traffic-pad`'s case a number that the Rust in one repo has to agree
   with.
5. The keycap. Move `margin-mail`'s `Key.tsx` and `Key.css`; calendar's `.key` is already the same
   chip, and margin-docs' `.key-cap` is a divergence that should be resolved rather than kept.
6. `keyLabel`, `normalizeCombo`, `PRIMARY_LABEL` and the `NAMED` map. Three near-identical
   implementations of the same twenty lines with three different bugs. `margin-mail`'s is the
   correct one. This is not strictly a UI primitive, but it is why the caps and titles disagree.

**Share, but the shape needs deciding first:**

7. The icon button. Eleven rules for one control is the clearest duplication in the audit, but
   `margin-mail`'s `Button` bundles size, variant, keycap and icon into one component, while the
   other three want a flat class they can put on any element. The tractable move is to share the
   CSS (a `.icon-button` at 28px with `--r-sm`, `--accent-wash` hover, and a settled `[data-on]`
   ring) and let each app keep its own JSX for now. Renaming margin's `.icon-btn` and settling on
   one of `data-on` or `data-active` is a prerequisite either way.
8. The `.titlebar` grid rule and the traffic lane. The nine declarations are common; the lane
   mechanism is not, and margin-docs' child-margin version is the correct one. `titlebar.rs`
   belongs in a shared Rust crate eventually, but that is a bigger move than this audit covers.

**Do not share:**

- Spinners and loading states. margin spins, calendar refuses to have any loading affordance,
  margin-mail bans spinners on the record. These are four different product positions, not four
  copies of one decision.
- Badges, chips and pills. The wash-chip recipe recurs, but every app's numbers are tuned to its
  own density (a calendar all-day chip is a layout unit, not a badge). Sharing the tokens is
  enough.
- Focus ring exceptions. The global rule is already shared through the tokens; the 18 `outline:
  none` sites are each local judgement calls, and margin-docs is the only app that writes down why.
- The floating editor toolbar. It exists in two apps only, and its `.tool` is a different control
  from `.icon-button` (a min-width pill that holds a letterform as often as a glyph). Worth
  de-duplicating between margin and margin-docs, not worth putting in a package the calendar and
  mail apps import.

**Prerequisite for all of it:** `margin-calendar` has to depend on `margin-shared`. It is one line
in its `package.json` (`"margin-shared": "file:../margin/shared"`) and deleting its hand-copied
`tokens.css` values. Every calendar-specific divergence in this document, `SEARCH`, `MORE`, `SUN`,
`MOON`, `LINK`, `REFRESH`, the duplicated palette, follows from the fact that it never joined.
