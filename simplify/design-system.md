# The design system: `@margin/tokens` and `@margin/fonts`

Scope: CSS custom properties, the base layer, themes and the six bundled faces. The React primitives
that consume them are in [ui-kit.md](ui-kit.md); the glyphs are `@margin/icons`. Short names:
`margin` = `python/margin`, `calendar` = `python/margin-caledar`, `editor` = `rust/margin-editor`,
`mail` = `rust/margin-mail`, all under `/Users/pj/Workspace/projects`. Line numbers as of 2026-09-06.

## Where the four token sets actually are

They are not four palettes. Calendar, the only app that does not depend on `margin-shared` at all and
so had every opportunity to drift, matches `shared/css/tokens.css` on 49 of the 52 values it declares
in common. Absent are `--font-book`, `--pane-sidebar`, `--measure` and `--sidebar`, which a calendar
has no use for and no sidebar to paint. One value genuinely differs, `--ink-faint`.

So this is not a reconciliation. Three apps already import the shared file
(margin/src/styles/tokens.css:3, editor/src/styles/tokens.css:5, mail/src/styles/tokens.css:6) and
layer on top; the fourth is a hand-copy. The work is to move twenty tokens that two or three apps
each declared separately into the one place, correct one value that is wrong in shared and right in
two apps, and delete four copies of a reset.

What drift exists is almost entirely in margin, the only app with no test suite. Colour literals
outside each app's token layer: margin 16 hex and 18 rgba, calendar 0 and 1 (a box-shadow at
app.css:507), editor 0 and 0, mail 0 and 0. Margin's are mostly tokens it never adopted, named under
each group below.

`--ink-faint` is the one that matters, because it is a legibility defect and not a preference.
Shared's `#9b9484` measures 2.91:1 on `--paper`, 2.56:1 on `--shell` and 2.43:1 on `--sidebar`, and
the dark `#756d5e` 3.39:1 on `--paper`. Calendar (tokens.css:73, :126) and mail (mail.css:86, :142)
independently moved to `#6e675b` and `#8e8677`, measuring 5.40 / 4.75 / 4.50 light and
4.81 / 5.14 / 4.51 dark. Margin and editor still ship the failing value.

## The contract for `@margin/tokens`

Fifty six names: thirty five are what `shared/css/tokens.css` declares today, twenty one are
promotions. Every one is declared in both palettes or in neither, because a token defined in light
and missing in dark is the failure editor's `src/theme.test.ts` catches, and that test moves into the
package.

### Colour

| Token | Light | Dark | Status |
| --- | --- | --- | --- |
| `--paper` | `#fcfbf7` | `#1d1a16` | shared |
| `--shell` | `#f1ece2` | `#151310` | shared |
| `--sidebar` | `#ece6da` | `#1a1713` | shared |
| `--raised` | `#fbfaf6` | `#232019` | shared |
| `--ink` | `#23201b` | `#ece6da` | shared |
| `--ink-soft` | `#6b6458` | `#a89f8e` | shared |
| `--ink-faint` | `#6e675b` | `#8e8677` | **corrected**, was `#9b9484` / `#756d5e` |
| `--line` | `#e3ddce` | `#2c2823` | shared |
| `--line-strong` | `#d6cfbd` | `#3a352d` | shared |
| `--accent` | `#2a2622` | `#efe9dc` | shared |
| `--accent-ink` | `#100e0b` | `#ffffff` | shared |
| `--accent-wash` | `rgba(35, 32, 27, 0.08)` | `rgba(239, 233, 220, 0.12)` | shared |
| `--accent-contrast` | `#faf7f0` | `#1a1714` | shared |
| `--selection` | `rgba(35, 32, 27, 0.14)` | `rgba(239, 233, 220, 0.18)` | shared |
| `--glass` | `rgba(252, 251, 247, 0.86)` | `rgba(33, 30, 25, 0.88)` | shared |
| `--scrim` | `rgba(35, 32, 27, 0.28)` | `rgba(0, 0, 0, 0.58)` | **promoted** |
| `--danger` | `#b4453a` | `#d97a6f` | shared |
| `--danger-ink` | `#963327` | `#e58e84` | shared |
| `--danger-wash` | `rgba(180, 69, 58, 0.1)` | `rgba(217, 122, 111, 0.16)` | shared |
| `--danger-contrast` | `#faf7f0` | `#1a1714` | shared |
| `--hue-1` to `--hue-8` | `#6f6194` `#47705f` `#a06044` `#4e6484` `#856434` `#955767` `#3a6b78` `#667141` | `#a396c8` `#7fae98` `#d19a7c` `#8ba2c4` `#c4a267` `#cb909e` `#75a9b6` `#a2ae77` | **promoted** |
| `--pdf-page` | `#fff` | `#fff` | **promoted**, identical in both on purpose |

`--ink-faint` is the only promotion that changes what margin and editor look like, and it will be
visible: every date stamp, word count, nav label and field label in both gets darker.

`--scrim`: calendar tokens.css:84/:138 and mail mail.css:88/:145 agree exactly; editor tokens.css:26
has `rgba(0, 0, 0, 0.5)` dark. Two to one, and both of the two wrote the reason down (a light scrim
over a dark page dims nothing, so panels float on shadow alone). Margin writes the light value into
`.overlay` at app.css:1275 and a second, unexplained `rgba(35, 32, 27, 0.32)` into `.export-overlay`
at app.css:1449; both become `var(--scrim)`.

The hue ramp is calendar's `--cal-1..8` (tokens.css:106-113, :160-167) and mail's `--hue-1..8`
(mail.css:114-121, :159-166), the same sixteen hexes under two names. Mail's wins, because `--cal-`
names a product; calendar renames on adoption, keeping its unrelated `--cal-h`, `--cal-sat` and
`--cal-fill` HSL derivations in grid.css. `--pdf-page` editor tokenised at tokens.css:22 because it
was a literal in two stylesheets; margin still has that literal at app.css:1238.

Not promoted, one consumer each: mail's `--check` / `--check-tick`, `--note-surface`, `--row-hover`,
`--row-selected`, `--card-ring`, `--message-*`; calendar's `--grid-*`, `--fold-*`, `--event-*`;
editor's `--code-*` and `--doc-rule*`; margin's twelve proofing colours at app.css:2367-2451. The
proofing pair is worth comparing separately, since editor's styles/proofing.css draws the same
feature out of `--danger`.

### Type scale

| Token | Value | Status |
| --- | --- | --- |
| `--font-ui` | `"Hanken Grotesk", ui-sans-serif, system-ui, -apple-system, sans-serif` | shared |
| `--font-book` | `"Literata", Georgia, "Times New Roman", serif` | shared |
| `--font-heading` | `"Literata", Georgia, "Times New Roman", serif` | shared |
| `--font-mono` | `ui-monospace, "SF Mono", Menlo, monospace` | **promoted** |
| `--t-1` | `11px` | shared |
| `--t-2` | `12px` | shared |
| `--t-3` | `13px` | shared |
| `--t-4` | `15px` | shared |
| `--t-5` | `16px` | **promoted** |
| `--measure` | `46em` | shared |

`--t-5` is identical in calendar tokens.css:34, editor tokens.css:13 and mail mail.css:17, and
calendar's and mail's comments are nearly word for word the same (iOS Safari zooms a focused field
under 16px and the locked viewport gives no way back). `--font-mono` is a token only in editor
(tokens.css:9); calendar writes `ui-monospace, SFMono-Regular, Menlo, monospace` as a literal at
details.css:183 and rich.css:42, so editor's wins as the one already tokenised.

`--font-ui`, `--font-book` and `--font-heading` are the three slots an app may override on the root
at runtime, and the contract says so: editor writes `--font-book` per document from
`store/useDocumentFonts.ts:101`, mail writes the other two from a setting in index.html:31-32. It is
why `.panel-head h2` takes `--font-heading` below.

### Spacing

There is no spacing scale in any of the four and this plan does not invent one. Padding and gap are
literals everywhere and inconsistent (panel bodies `20px 22px 24px`, heads `16px 14px 16px 22px`,
feet `14px 22px`), and what does repeat repeats inside a rule that is itself being shared. A scale
imposed now would be a rewrite of eighteen thousand lines of CSS with no defect behind it.

### Radius

| Token | Value | Status |
| --- | --- | --- |
| `--r-sm` | `5px` | shared |
| `--r-md` | `8px` | shared |
| `--r-lg` | `12px` | shared |
| `--r-pill` | `999px` | **promoted** |

`--r-pill` is identical in calendar tokens.css:8, editor tokens.css:10 and mail mail.css:13: four
apps, one value, three declarations and nine literals in the fourth.

### Shadow

| Token | Light | Dark | Status |
| --- | --- | --- | --- |
| `--shadow-page` | `0 1px 2px rgba(35, 32, 27, 0.06), 0 14px 30px rgba(35, 32, 27, 0.09)` | `0 1px 2px rgba(0, 0, 0, 0.4), 0 14px 34px rgba(0, 0, 0, 0.5)` | shared |
| `--shadow-pop` | `0 8px 24px rgba(35, 32, 27, 0.14)` | `0 8px 24px rgba(0, 0, 0, 0.55)` | shared |
| `--shadow-raised` | `0 1px 2px rgba(35, 32, 27, 0.05)` | `0 1px 2px rgba(0, 0, 0, 0.35)` | **promoted** |

Only editor declares `--shadow-raised` (tokens.css:15, :27); margin writes the light value literally
at app.css:272 and :1839, and calendar's one rgba literal, `0 1px 2px rgba(0, 0, 0, 0.06)` at
app.css:507, is a fourth alpha for the same effect. Margin's `.card:hover` shadow at app.css:1547 is
a two layer lift neither token expresses and stays a literal.

### Motion

| Token | Value | Status |
| --- | --- | --- |
| `--ease` | `cubic-bezier(0.22, 0.61, 0.36, 1)` | shared |
| `--dur` | `120ms` | **new** |

`--dur` is the one name here that exists nowhere today, and it is a promotion of a measured constant
rather than an invented scale: `120ms` appears 54 times in margin's CSS, 64 in calendar's, 83 in
editor's and 58 in mail's, and every other duration in all four together under 40 times. The shared
sheets use it; app sweeps happen when somebody next touches the rule.

### Layout metrics

| Token | Value | Status |
| --- | --- | --- |
| `--titlebar-h` | `46px` | shared |
| `--pane-sidebar` | `248px` | shared |
| `--traffic-pad` | `84px` | **promoted** |
| `--touch-h` | `44px` | **promoted** |
| `--phonebar-h` | `48px` | **promoted** |
| `--tabbar-h` | `56px` | **promoted** |
| `--sheet-max-h` | `88dvh` | **promoted** |

All five promotions are byte-identical where they exist, with near-identical comments:
`--traffic-pad` at calendar tokens.css:11, editor tokens.css:11, mail mail.css:36; `--touch-h` at
calendar tokens.css:22, editor tokens.css:12, mail mail.css:31; the phone trio at calendar
tokens.css:16-26 and mail mail.css:26-34.

Margin has no `--traffic-pad` and hardcodes the lane at app.css:75, `padding: 0 14px 0 84px` on
`.titlebar`, with no `data-traffic` gate. The other three gate it (calendar app.css:126, editor
app.css:97, mail screens/header.css:24) and calendar's comment at app.css:121-125 says what the
ungated version costs: 84px of nothing on Linux, Windows, an iPad and a browser, pushing the centred
title off centre. Margin also redefines `--titlebar-h` at app.css:2881 as
`calc(46px + env(safe-area-inset-top))`; it becomes `calc(var(--titlebar-h) + var(--safe-top))`,
which is calendar's `.titlebar` at app.css:106.

Not promoted: `--row-h`, a 48px calendar grid row (calendar tokens.css:44) and a 46px message list
row (mail mail.css:45). One name, two meanings, and the clearest reason not to promote geometry.

### Safe area

| Token | Value | Status |
| --- | --- | --- |
| `--safe-top` | `env(safe-area-inset-top, 0px)` | **promoted** |
| `--safe-bottom` | `env(safe-area-inset-bottom, 0px)` | **promoted** |

Calendar tokens.css:16-17 and mail mail.css:26-27, identical. The `0px` fallback is what makes a
desktop read both as zero without a media query; margin and editor read `env()` inline.

## The base layer

Parsing each app.css into selector and body pairs and comparing bodies exactly, five rules are
byte-identical in all four with no exceptions: `*` (`box-sizing: border-box`), `html, body, #root`
(`height: 100%`), the eleven line `body` block, `::selection`, and `:focus-visible`, the same three
lines at margin app.css:43, calendar app.css:80, editor app.css:51 and mail app.css:92.

`@margin/tokens` ships `css/tokens.css`, `css/base.css` and `css/overlay.css` plus a `css/index.css`
importing all three in that order. `index.css` is what an app imports, first, before anything else;
the split exists so the audit script can parse the token file alone, not so an app can pick.

`css/base.css` holds those five plus the rules below, taking the version named:

| Rule | Winner | What the losers are missing |
| --- | --- | --- |
| `html` | calendar app.css:37, mail app.css:23 | `-webkit-tap-highlight-color: transparent`, without which a webview reads every tap as the start of a selection |
| `button` | mail app.css:70 | `font-size: inherit` (margin app.css:34, calendar app.css:63, editor app.css:34 are otherwise identical) |
| `input, textarea, select` | calendar app.css:72, editor app.css:43, mail app.css:80 | margin has no such rule, so its fields take the webview's own font |
| `svg { display: block }` | mail app.css:88 | the inline baseline gap under every glyph, patched per component in the other three |
| `.app` | calendar app.css:86, editor app.css:58, mail app.css:100 | margin app.css:59 still uses `height: 100vh; height: 100dvh` |
| `:root[data-touch]` selection rules, `:root[data-phone]` field size | calendar app.css:44-60, mail app.css:44-61 | absent in margin and editor |
| `::-webkit-scrollbar` and its three parts | margin app.css:693-712 | the other three take the webview default |

Two need a sentence. On `.app`, calendar's comment at app.css:89-97 explains that UIKit shrinks the
layout viewport by the safe areas while leaving it anchored at y 0, so a viewport unit lays the tab
bar out below the clip `body { position: fixed }` imposes, where it is invisible and untappable. The
scrollbar is 11px, thumb in `--line-strong` with a 3px transparent border and
`background-clip: content-box`, hover `--ink-faint`, transparent track.

`css/overlay.css` holds the box every app builds its dialogs from, the largest near duplicate in the
four codebases. `.overlay` is byte-identical at calendar app.css:330, editor app.css:451 and mail
app.css:110, and margin app.css:1271 is the same rule with the scrim written out. `.panel` and
`.panel-body` are byte-identical in all four (margin app.css:1283, calendar app.css:342, editor
app.css:463, mail app.css:129). `.panel-foot` is identical at margin app.css:1622, calendar
app.css:415 and editor app.css:502, and mail app.css:168 adds `flex: none`, which ships.
`.overlay[data-align="top"] { align-items: start; padding-top: 12vh }` appears at calendar
palette.css:4, editor palette.css:9 and mail app.css:124; margin has no palette and gains an inert
rule.

Two reconciliations in the head. Editor's `padding: 16px 16px 16px 22px` beats the other three's
`16px 14px 16px 22px` on editor's own argument at app.css:474-478: on the closing end sits a 28px
icon button carrying a 16px glyph 6px in from its own edge, so 16 plus 6 puts the cross on the same
right edge as the body text and the footer buttons, and all four put a close button there. And
`.panel-head h2` takes `var(--font-heading)`, as calendar app.css:401 and mail app.css:152 do, not
`var(--font-book)` as margin app.css:1301 and editor app.css:489 do. Both resolve to Literata today,
but editor overwrites `--font-book` on the root per document, so a document set in EB Garamond
currently retitles every dialog in EB Garamond, and a dialog title is chrome. Mail's other head
extras ship: `flex: none`, `gap: 10px`, and `flex: 1; min-width: 0` on the `h2` so a long title
truncates rather than shoving the close button off the edge.

Phone docking is written twice today, identically and with the same comments, at calendar
app.css:353-390 and mail app.css:174-212: `:root[data-phone]` variants of `.overlay` (z-index 50,
padding 0, `place-items: end center`), `.panel` (full width, `--sheet-max-h`, top border only, radius
`var(--r-lg) var(--r-lg) 0 0`, `padding-bottom: var(--safe-bottom)`, `animation: sheet-up 180ms
var(--ease)`) and `.panel-body` (`overscroll-behavior: contain`), plus `@keyframes sheet-up`. All of
it ships, including mail's `@media (prefers-reduced-motion: reduce)` guard, which calendar lacks:
mail has 11 such guards, editor 1, calendar 0 and margin 0, while calendar and margin both animate
with nothing. Every keyframe in the shared sheets carries a guard, and the apps inherit that with
it.

## Theming

`data-theme` on the root, written from JavaScript, never a `@media (prefers-color-scheme)` rule in
CSS. All four already do this: one decision taken four times, and the right one, since the system
preference is an input to the setting rather than the setting itself and a user who picked light at
four in the afternoon keeps it.

`color-scheme` is declared, in both shared palette blocks: `light` on
`:root, :root[data-theme="light"]` and `dark` on `:root[data-theme="dark"]`. Only editor has it today
(themes.css:24-31), which is why the caret, the native form controls and the default scrollbar come
out light on a dark page in the other three. Calendar's workaround at create.css:53 and :57 is
deleted as redundant. Mail's rules on `.msg-frame` at screens/pane.css:249-261 stay: an embedded
document takes its scheme from the element embedding it, and a sender's HTML is pinned `only light`.

The multi-palette design stays local to editor, which ships seven palettes (`light`, `sepia`, `mist`,
`contrast`, `dark`, `graphite`, `midnight`) in a 323 line themes.css, each block declaring the same
thirty five properties and enforced by `src/theme.test.ts`. Generalising it means every app's
stylesheet answering `sepia` and `midnight` or falling through to the light `:root` block, and for
calendar that is seven variants each of `--grid-*`, `--fold-*`, `--event-*` and the hue ramp, roughly
140 new declarations for a feature it has not asked for. The mechanism is shared, not the palettes,
and an app that later wants sepia adds a block and a row in its list.

One piece of themes.css does move. Lines 307-322 copy six hexes of the shared palettes so the
picker's preview tiles can draw them, with a comment saying they are copied because "this repo may
not reach into" margin-shared. It can, and does. The two shared palette blocks gain
`.theme-swatch[data-theme="light"]` and `.theme-swatch[data-theme="dark"]` to their selector lists,
which gives a swatch all twenty colours rather than six and deletes both blocks.

The hook lives in `@margin/hooks`, per [repo-layout.md](repo-layout.md), and covers both shapes:

    useTheme<T extends string>(options: {
      key: string;                 // "marginmail-theme"
      themes: readonly T[];        // ["light", "dark"], or editor's seven
      schemeOf?: (id: T) => "light" | "dark";
    }): { theme: T; choice: T | "system"; scheme: "light" | "dark";
          setChoice(next: T | "system"): void }

It reads `data-theme` off the root first (the boot script has already run), then storage, then
`matchMedia`. Storage goes through try/catch, because a webview with storage denied should still
theme and merely forget between launches; editor's theme.ts:105-122 has that and the other three do
not. It remembers a light half and a dark half so `"system"` lands on the palettes the user actually
picks, and registers the `matchMedia` listener. Margin, calendar and mail pass a two element list and
get what they have now plus the storage guard; editor passes seven and its `Preference` type goes.

The boot script is the other half and cannot import a bundle, so `@margin/hooks` exports
`themeBootScript(options)` returning the source as a string, injected by each app's Vite config
through `transformIndexHtml`. It writes `data-theme`, `data-phone` and `data-touch`, the part
identical across calendar index.html:12-22 and mail index.html:12-22, and each app appends its own
lines: calendar's `data-view`, mail's `data-no-pane` and font slots, editor's `data-sidebar` and
`data-width`, margin's `--measure` and pane widths.

## Fonts

`@margin/fonts` is today's `shared/src/fonts.ts` (233 lines: `BUNDLED_FONTS`, `FontRef`, `FontPair`,
`FONT_PAIRINGS`, `fontStack`, `fontsUsed`), `shared/css/fonts.css` (twelve `@font-face` rules over
six families), `shared/fonts/` (twelve variable TTFs and six OFL notices, 13.8 MB) and
`shared/bin/sync-fonts.mjs`, moved intact. Nothing about it is wrong. All 18 files in `shared/fonts`
are byte-identical to the copies in margin, editor and mail's `public/fonts` (verified 18/18 each
with `cmp`), and calendar's 4 are byte-identical too.

The vendored copies stay and the reason narrows. Margin's `src-tauri/src/pdf.rs:9-20` reads all
twelve variable files with `include_bytes!` from `../../public/fonts`, so the bytes must be at a path
cargo can see before any npm install has run. Editor no longer does: `src-tauri/src/pdf.rs:33-40`
reads ten static instances out of `src-tauri/fonts`, and calendar and mail read no fonts from Rust at
all. The package's comment saying "both apps' pdf.rs" is out of date by one app; the copies now exist
for margin's exporter and for the webview, which serves them from `/fonts/Literata-VF.ttf`.

`sync-fonts` keeps its behaviour, which is right: copy every `.ttf` and `.txt`, or with `--check`
compare and exit 1 on any difference, reporting rather than deleting a file the app has and the
package does not (lines 53-59). It gains one flag, `--core`, restricting the set to Literata and
Hanken Grotesk and their two OFL notices, and `@margin/fonts` exports two stylesheets to match:
`css/fonts.css` with all twelve faces and `css/core.css` with the four. Otherwise an app declares
faces it does not vendor and `fonts:check` passes against a `@font-face` pointing at a 404.

Calendar is what changes. It has no dependency on the package, no `fonts:sync` or `fonts:check`, no
way to notice drift, and 31 lines of hand-written `@font-face` in `src/styles/fonts.css` that are
character for character `shared/css/fonts.css:17-47`. It ships no OFL notices, which is the one real
problem here, since shipping a face without its licence is a licensing defect. It takes the
dependency, replaces fonts.css with `@import "@margin/fonts/css/core.css"` and adds the two scripts
running `--core`, gaining two OFL notices and 4 KB rather than 11.7 MB it has no picker to offer.

## Per app

**margin.** Deletes tokens.css:1-3 (the import moves to `@margin/tokens/css/index.css`), keeping the
file for `--pane-dock: 384px` alone, and deletes `src/styles/fonts.css` for
`@margin/fonts/css/fonts.css`. Deletes app.css:1-58 (the reset, keeping the two app-specific
`:focus-visible` overrides at :47-57), :59-65 (`.app`), :693-712 (the scrollbar, which moves up),
:1271-1316 and :1622-1628 (overlay and panel), about 110 lines. Adopts `--scrim`, `--shadow-raised`,
`--r-pill`, `--traffic-pad`, `--safe-top`, `--pdf-page` and `--dur` in place of 30-odd literals, and
gains the `data-traffic` gate it never had. `src/theme.ts` goes for the hook. This app changes most,
being the one that never adopted the last three rounds of shared work.

**calendar.** Deletes tokens.css:1-36 and :63-146 (the `:root` block and both palettes), keeping the
grid geometry and the `--grid-*`, `--fold-*` and `--event-*` blocks: 84 of 168 lines. Renames
`--cal-1..8` to `--hue-1..8` at its 20-odd call sites. Deletes `src/styles/fonts.css`, app.css:1-99
and :329-420, plus create.css:53 and :57. Gains a shared dependency for the first time, plus
`fonts:sync --core` and `fonts:check --core`, and two OFL notices in `public/fonts`.

**editor.** Deletes tokens.css:8-28 (the shape and scale block, `--scrim`, `--shadow-raised`,
`--pdf-page`), keeping the document scale and code surface from :30 down: 21 of 108 lines. Deletes
app.css:1-62 and :451-508, and themes.css:24-31 and :296-322. Its `src/theme.test.ts` moves into
`@margin/tokens` as the audit script and gains the other three apps as inputs; `src/theme.ts`
collapses into the hook, keeping only `THEMES` and the labels. `src-tauri/fonts` is untouched: a
typesetting decision, in [typesetting.md](typesetting.md).

**mail.** Deletes `src/styles/app.css` entirely, all 212 lines, the cleanest single deletion in this
plan: the file is the reset, the window, the overlay and the phone docking and nothing else. Deletes
mail.css:10-36 (`--r-pill`, `--t-5`, phone chrome, `--traffic-pad`) and :86-88 and :142-145 (the
`--ink-faint` and `--scrim` overrides, now upstream), keeping mail geometry, the row surfaces, the
note surface, `--check`, `--card-ring` and `--message-*`: 120 of 167 lines. tokens.css becomes a two
line seam and `src/theme.ts` goes for the hook.

Every app keeps exactly one app-local token file, and the rule mail's seam already states becomes the
family rule: nothing outside the token layer may declare a token, and nothing outside it may write a
literal colour, radius or size.

## Order, and what proves each step

Nothing starts until mail and editor's 123 uncommitted files each are committed and mail has a
remote. [risks.md](risks.md) treats that as a precondition and it is.

**1. Correct the values in place.** Change `--ink-faint` in `python/margin/shared/css/tokens.css` and
add the twenty other promotions to that same file, before anything moves repository. The dependency
is still a symlink here, so margin, editor and mail see it on the next dev restart and each visual
diff has exactly one cause. Check: run the contrast assertion over every ink-on-surface pair in both
palettes and require 4.5:1; screenshot margin's chapter list and editor's document tree in both
themes and confirm the only difference is faint ink getting darker.

**2. Land the audit script.** Promote editor's `src/theme.test.ts` into the package. It parses the
stylesheets rather than rendering them, builds the set of properties each palette declares, and fails
when a block is short or a `data-theme` block has no `color-scheme`. Extend it with the literal lint
risks.md asks for: any hex or `rgba(` outside the token layer is a failure. Check: it reports 16 hex
and 18 rgba in margin, 1 rgba in calendar, 0 and 0 in editor and mail. After step 5, four zeroes.

**3. Calendar joins.** Add the dependency, delete its palette blocks, import the shared tokens,
rename `--cal-*`, take `core.css` and the two font scripts. Check: the audit reports zero drift
between calendar and shared; `pnpm fonts:check --core` passes; `pnpm test:ui` passes, in particular
`tests/legibility.spec.ts`, which renders the week grid in both themes and would catch an hour axis
that lost its contrast. Compare the week grid on a Monday with overlapping events.

**4. Extract the base sheet.** `css/base.css` and `css/overlay.css`, one app at a time in the order
mail, editor, calendar, margin: mail first because its version is the superset and its app.css is a
clean delete, margin last because it needs the most reconciliation and has no suite to catch a
mistake. Check per app: mail `tests/shell.spec.ts` and `tests/kit.spec.ts`; editor
`tests/smoke.spec.ts` and `tests/_rule.spec.ts`; calendar `tests/overlays.spec.ts`,
`tests/phone.spec.ts` and `tests/touch.spec.ts`. The screen to compare is a dialog over the main view
at 1280 wide and at 390: mail's Settings sheet over the thread list, editor's Document Setup over an
open document, calendar's quick create over the week, margin's Settings panel over the chapter list.
Margin has no Playwright config, so its check is a manual pair of screenshots per theme; giving it a
suite belongs in [testing.md](testing.md) and should happen before this step rather than after.

**5. Land the scrollbar and `--dur`.** Separately from step 4, because the scrollbar is the one base
rule that changes layout: an 11px classic scrollbar where the webview drew an overlay one takes 11px
from whatever it is inside. On macOS it only shows under "Show scroll bars: Always", which is why
nobody noticed margin had it. Check: screenshot every scrolling surface with that setting forced on,
specifically mail's virtualised list, calendar's grid and editor's tree.

**6. Rename and repackage.** `margin-shared` becomes `@margin/tokens` and `@margin/fonts` in the new
repository and all four apps move from a `file:` path to a version. This changes no CSS and should
produce no visual diff at all, which is the point of doing it last. Check: the audit and all three
suites pass unchanged, and `pnpm install` succeeds in a fresh clone of each app with no sibling
checkout present, which is the failure that motivated the whole exercise.

## What stays per app

Editor keeps its document scale (`--doc-h1` through `--doc-h6`, `--doc-size`, `--doc-lead`), its four
measures, its sheet padding, its code surface and its six syntax hues, and it keeps themes.css. Those
51 tokens describe a page of prose, and a calendar and a mail client have no page.

Mail keeps its geometry (`--list-w`, `--row-h`, `--avatar`, `--pile-h`, `--compose-w`, `--feed-w`),
its row surfaces, `--note-surface`, `--check`, `--card-ring` and the five `--message-*` values. The
`--message-*` set must not be promoted: it is the light palette's values written a second time,
deliberately outside both theme blocks, because HTML mail is authored for a white page and a
newsletter that sets a dark text colour and no background is unreadable on a dark surface. A shared
token that ignores the theme would contradict the contract.

Calendar keeps its grid geometry and the `--grid-*`, `--fold-*` and `--event-*` palettes, plus the
per-event HSL derivations at grid.css:184-223, a colour system inside a colour system that exists
because a calendar draws dozens of tinted blocks at once. Margin keeps `--pane-dock: 384px`, the
proofing colours at app.css:2367-2451, and the device frame's `--dv-*` set, which comes from
`components/DeviceFrame.tsx:22-24` rather than from CSS at all.

And the thing not to share at all: geometry by name. A shared `--row-h` would be a token every
consumer overrides, which is a name with no value in it, and the next reader would reasonably assume
the calendar grid and the mail list were meant to line up.
