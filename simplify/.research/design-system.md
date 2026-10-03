# The visual design system across the four apps

Scope: CSS only. Tokens, fonts, the base layer, themes, and the layout idioms that repeat.

Short names below: `margin` = /Users/pj/Workspace/projects/python/margin, `calendar` =
/Users/pj/Workspace/projects/python/margin-caledar, `editor` =
/Users/pj/Workspace/projects/rust/margin-editor, `mail` = /Users/pj/Workspace/projects/rust/margin-mail.
Between them: margin 3 CSS files and 2983 lines, calendar 11 and 3796, editor 21 and 5089, mail 47
and 6571. 18439 lines total.

## What margin-shared already is, and who takes it

`/Users/pj/Workspace/projects/python/margin/shared` ships `css/tokens.css` (35 distinct custom
property names), `css/fonts.css` (12 `@font-face` rules, 6 families), `fonts/` (18 files: 12 TTFs
and 6 OFL notices), `src/fonts.ts`, `src/icons.ts` and `bin/sync-fonts.mjs`.

Three of the four consume it. margin declares `"margin-shared": "file:./shared"`, editor and mail
both declare `"file:../../python/margin/shared"`. Calendar does not depend on it at all and does not
import either stylesheet.

The seams are thin and consistent in the three that do:

- margin/src/styles/tokens.css:3 imports the shared tokens, then adds exactly one line, `--pane-dock: 384px`.
- editor/src/styles/tokens.css:5 imports, then adds 51 tokens of its own (document scale, sheet padding, code surface).
- mail/src/styles/tokens.css:6 imports, then imports `./mail.css`, which adds 46.

## Tokens

### Calendar is a 49-of-52 copy of the shared file

calendar/src/styles/tokens.css is not a divergent palette. Comparing it block for block against
shared/css/tokens.css:

- `:root`: 11 of 11 comparable values byte-identical (`--font-ui`, `--font-heading`, `--r-sm/md/lg`,
  `--titlebar-h`, `--t-1` through `--t-4`, `--ease`). Absent: `--font-book`, `--pane-sidebar`,
  `--measure`, which a calendar has no use for.
- light block: 19 of 20 identical. One drift.
- dark block: 19 of 20 identical. Same one drift.
- Absent from both palettes: `--sidebar`, correctly, since the grid owns the window and there is no sidebar.

The one drift is `--ink-faint`. Shared has `#9b9484` light and `#756d5e` dark; calendar
(tokens.css:74, :126) and mail (mail.css:86, :142) both have `#6e675b` and `#8e8677`.

Two apps independently moved the same token to the same two values for the same stated reason
(4.5:1 contrast on the surfaces faint ink lands on; the calendar comment names the hour axis, the
mail comment names list times and snippets and says "same reasoning, same value, as the calendar's
hour axis"). Editor and margin still take `#9b9484`. That is not two apps needing to differ, it is
the shared value being wrong and two apps finding out separately. Move the pair upstream and delete
both overrides.

### Tokens defined in more than one app under different names, or defined in one and hardcoded in another

- `--scrim`. Not in shared. Light is `rgba(35, 32, 27, 0.28)` in all three that have it (calendar
  tokens.css:84, editor tokens.css:14, mail mail.css:88). Dark: calendar and mail `rgba(0, 0, 0, 0.58)`,
  editor tokens.css:26 `rgba(0, 0, 0, 0.5)`. margin has no token and writes the light literal into
  `.overlay` at app.css:1275 and a second, different one, `rgba(35, 32, 27, 0.32)`, into
  `.export-overlay` at app.css:1449. This belongs in shared.
- `--shadow-raised`. editor tokens.css:15 `0 1px 2px rgba(35, 32, 27, 0.05)`, dark
  `0 1px 2px rgba(0, 0, 0, 0.35)`. margin writes that light value as a literal twice, app.css:272
  and app.css:1839.
- `--r-pill: 999px`. Declared separately in calendar tokens.css:8, editor tokens.css:10 and mail
  mail.css:12, identically. margin writes `border-radius: 999px` as a literal at app.css:2184. Four
  apps, one value, three declarations and one literal.
- `--t-5: 16px`. calendar tokens.css:34, editor tokens.css:12, mail mail.css:17. Identical, and the
  comment in calendar and mail is nearly word for word the same (iOS zooms a field under 16px).
- `--touch-h: 44px`. calendar tokens.css:22, editor tokens.css:11, mail mail.css:31. Identical.
- `--traffic-pad: 84px`. calendar tokens.css:11, editor tokens.css:11, mail mail.css:36. Identical;
  margin has no token and hardcodes the lane unconditionally, see the drift section.
- `--safe-top` / `--safe-bottom` / `--phonebar-h: 48px` / `--tabbar-h: 56px` / `--sheet-max-h: 88dvh`.
  calendar tokens.css:16-26 and mail mail.css:26-34, identical values and near-identical comments.
  A five-token phone chrome block written twice.
- calendar `--cal-1` through `--cal-8` (tokens.css:106-113 light, :160-167 dark) and mail `--hue-1`
  through `--hue-8` (mail.css:115-121, :159-166) are the same sixteen hexes under two names. mail's
  own comment says so: "the calendar's --cal-1..8 under a name that says what they are for here".

### App-only tokens that should stay app-only

editor's 51 additions are document typography and sheet geometry (`--doc-h1` through `--doc-h6`,
`--measure-*`, `--sheet-pad-*`, `--code-*`, `--pdf-page`). mail's 46 are mail geometry (`--list-w`,
`--avatar`, `--pile-h`, `--compose-w`, `--feed-w`, the `--message-*` set that deliberately does not
follow the theme). Calendar's are grid geometry (`--gutter-w`, `--daybar-h`, `--strip-h`,
`--event-*`, `--grid-*`, `--fold-*`). margin's is `--pane-dock: 384px`. All genuinely single-app.

Note the collision: `--row-h` means a calendar grid row (48px, calendar tokens.css:44) in one app and
a message list row (46px, mail mail.css:45) in the other. A reason not to promote geometry by name.

## Fonts

The bytes are already correct. All 18 files in shared/fonts are byte-identical to the copies in
margin/public/fonts, editor/public/fonts and mail/public/fonts (verified with `cmp`, 18/18 each).
Calendar vendors only 4 of them, `HankenGrotesk-VF.ttf`, `HankenGrotesk-Italic-VF.ttf`,
`Literata-VF.ttf`, `Literata-Italic-VF.ttf`, and those 4 are byte-identical to shared too. It ships
no OFL notices, which is the one real problem here: the other three ship all six.

`shared/bin/sync-fonts.mjs` copies every `.ttf` and `.txt` from shared/fonts into
`<app>/public/fonts`, or with `--check` compares and exits 1 on any difference; a file present in
the app and absent from the package is reported and left alone rather than deleted (lines 53-59).
The vendored copies exist because both PDF exporters read the same paths with `include_bytes!`, so
cargo must not wait on an npm install.

Who runs it: margin and editor as `node node_modules/margin-shared/bin/sync-fonts.mjs .`, mail as
`margin-shared-fonts .` through the package's `bin` entry. Calendar has no `fonts:sync` or
`fonts:check` script and no way to notice drift.

margin, editor and mail's src/styles/fonts.css are each a comment and one
`@import "margin-shared/css/fonts.css"`; margin's and editor's are byte-identical including the
comment. calendar/src/styles/fonts.css is 31 lines of hand-written `@font-face` for the four faces
it vendors, character-for-character the same as shared/css/fonts.css:17-47. Calendar joining costs
one import, one script pair, and 8 more files in public/fonts.

## The base layer in app.css

All four start with the same reset. Measured by parsing each app.css into selector/body pairs and
comparing bodies exactly:

- margin x editor: 71 selectors in common, 59 with byte-identical bodies.
- calendar x mail: 22 in common, 16 identical.
- calendar x editor: 23 in common, 15 identical.
- margin x calendar: 20 in common, 11 identical.
- editor x mail: 15 in common, 10 identical.
- margin x mail: 15 in common, 7 identical.

Identical in all four, no exceptions: `*`, `html, body, #root`, `body`, `::selection`,
`:focus-visible`. The focus ring is the same three lines everywhere (margin app.css:43, calendar
app.css:80, editor app.css:51, mail app.css:92):

```css
:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
```

And the body block, identical in all four (margin app.css:16, calendar app.css:16, editor app.css:16,
mail app.css:26):

```css
body {
  margin: 0;
  position: fixed;
  inset: 0;
  overflow: hidden;
  overscroll-behavior: none;
  background: var(--shell);
  color: var(--ink);
  font-family: var(--font-ui);
  font-size: var(--t-3);
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
}
```

Near-duplicates, with the differences named:

- `button`. margin app.css:34, calendar app.css:63 and editor app.css:34 are identical seven-line
  blocks. mail app.css:70 adds one line, `font-size: inherit`. That line is right and the other
  three are missing it.
- `html`. margin and editor stop at `text-size-adjust`. calendar app.css:37 and mail app.css:23 both
  add `-webkit-tap-highlight-color: transparent` with the same three-line comment about a webview
  reading every tap as a text selection. Two apps have the fix, two do not.
- `input, textarea, select`. Identical in calendar app.css:72, editor app.css:43, mail app.css:80.
  margin does not have it at all, so its fields fall back to the webview's font.
- `.app`. calendar app.css:86, editor app.css:58 and mail app.css:100 are identical
  (`display:flex; flex-direction:column; height:100%; overflow:hidden`). margin app.css:59 uses
  `height: 100vh; height: 100dvh`, which the other three have deliberately moved away from; the
  comment at calendar app.css:89-97 explains why.
- `:root[data-touch] .icon-button`, `.icon-button`, `.icon-button:hover`: identical between calendar
  app.css:145-170 and editor app.css:138-165. margin calls the same control `.icon-btn` and draws it
  30px instead of 28px (app.css:103). mail folded it into `.button[data-icon-only]` (ui/Button.css:33).
  Four apps, one control, three names and two sizes.

`::-webkit-scrollbar` exists in exactly one app, margin app.css:693-712 (11px, thumb `--line-strong`
with a 3px transparent border and `background-clip: content-box`, hover `--ink-faint`, transparent
track). The other three take the webview default. Editor gets the temperature right a different way,
via `color-scheme` in themes.css. Only editor declares `color-scheme` at the root
(themes.css:24-31 and once per palette); calendar declares it on one element,
create.css:53 and :57, on the quick-create card.

`@media (prefers-reduced-motion: reduce)`: mail has 11 (app.css:208, Banner.css:64, pane.css:297 and
:358, arriving.css:78, search.css:36, list.css:126, tour.css:29, contacts.css:56, :215, :306), editor
1 (export-preview.css:167), calendar 0, margin 0. Calendar and margin both animate
(`quick-create-in`, `sheet-up`, `find-drop`, `drawer-in-right`, `spin`) with no guard.

## The overlay and panel shell

The same box in all four, and it is the single largest near-duplicate in the codebase.

`.overlay` is identical in calendar app.css:330, editor app.css:451 and mail app.css:110:
`position:fixed; inset:0; z-index:20; background:var(--scrim); backdrop-filter:blur(2px);
display:grid; place-items:center; padding:40px`. margin app.css:1271 is the same rule with
`background: rgba(35, 32, 27, 0.28)` written out instead of a token.

`.panel` is byte-identical in all four (margin app.css:1283, calendar app.css:342, editor app.css:463,
mail app.css:129): `width: min(480px, calc(100vw - 32px)); max-height:100%; flex column; --paper;
1px --line; --r-lg; --shadow-pop; overflow:hidden`.

`.panel-body` is byte-identical in all four. `.panel-foot` is identical in margin, calendar and
editor; mail adds `flex: none`. `.panel-head`: margin, calendar and mail use `padding: 16px 14px 16px 22px`,
editor uses `16px 16px 16px 22px` with a comment at app.css:475-478 explaining the two pixels; mail
also adds `flex: none`, `gap: 10px` and a `flex: 1; min-width: 0` on the `h2`. `.panel-head h2` is
`font-family: var(--font-book)` in margin and editor and `var(--font-heading)` in calendar and mail,
which is a real fork: shared/css/tokens.css sets both to Literata by default, but editor lets a
document override `--font-book` at runtime and mail lets a setting override `--font-heading`, so the
same declaration means different things.

Phone docking is written twice, identically, comments included. calendar app.css:356-390 and mail
app.css:179-206 both carry `:root[data-phone] .overlay` (z-index 50, padding 0,
`place-items: end center`), `:root[data-phone] .panel` (full width, `--sheet-max-h`, border-width
`1px 0 0`, radius `var(--r-lg) var(--r-lg) 0 0`, `padding-bottom: var(--safe-bottom)`,
`animation: sheet-up 180ms var(--ease)`), `:root[data-phone] .panel-body { overscroll-behavior: contain }`
and `@keyframes sheet-up`. mail adds the reduced-motion guard, calendar does not.
`.overlay[data-align="top"] { align-items: start; padding-top: 12vh }` appears three times: calendar
palette.css:4, editor palette.css:9, mail app.css:124.

Panel sizes are the same idiom under two names: calendar overlays.css:9-20
`.overlay-panel[data-size="mini"]` at `min(292px, calc(100vw - 32px))` and `wide` at 560px, mail
Sheet.css:4-15 `.sheet[data-size="mini"]` at the identical 292px and `wide` at 620px. Both give mini
the same `.panel-body { gap: 10px; padding: 12px 12px 14px }`.

## Buttons and fields

Three of the four have converged on the same text button by three different routes.

calendar overlays.css:60-135 `.panel-button` and mail ui/Button.css:1-98 `.button` are the same
control: `inline-flex`, `gap: 7px`, `1px solid var(--line-strong)`, `var(--r-sm)`, `var(--raised)`
ground, `--accent-wash` hover, and `[data-variant="primary" | "danger" | "ghost"]` with the same
bodies (primary is accent ground with `--accent-contrast` ink hovering to `--accent-ink`; danger is
`--danger-ink` text hovering to `--danger-wash` with a `--danger` border; ghost is transparent border
and `--ink-soft`). The differences are the selector, the sizing (calendar pins `min-height: 30px`,
mail has `[data-size="sm|md|lg"]` at 26/28/32) and mail's `[data-icon-only]` square. Both end with
`:root[data-touch] { min-height: var(--touch-h) }`. editor settings.css:319 `.btn-quiet` and margin
app.css:1769-1805 `.btn-primary` / `.btn-ghost` / `.btn-danger` are a third and fourth spelling of
the same three variants; margin's danger is filled rather than outlined and it pads `9px 20px`.

Fields: calendar overlays.css:140-216 (`.field-input`, `.field-select`, `.field-textarea`,
`.field-hint`, `.field-check`) and mail ui/Field.css:18-59 are the same rules to within the padding
(`6px 9px` vs `7px 10px`) and mail's added `:disabled` and `::placeholder` blocks. Both hover to
`border-color: var(--ink-faint)`. margin app.css:1333-1348 is a third version on `.field input,
.field select` with `padding: 9px 11px` and a `:focus { border-color: var(--accent) }` the other two
lack. `.field` and `.field-label` are byte-identical between margin app.css:1318-1331, calendar
app.css:423-435 and mail ui/Field.css:1-16 (uppercase, `--t-1`, 600, `0.08em`, `--ink-faint`), which
is also exactly mail's `.group-head` (ui/GroupHead.css:1-11) and margin's and editor's `.nav-label`.
mail duplicates its own field twice more, at screens/settings.css:448-471 and :306.

The switch: editor settings.css:343-380 `.switch` / `.switch-knob` and mail ui/Toggle.css:1-40
`.toggle` / `.toggle-knob` are the same control at two sizes (38x22 with a 16px knob travelling 16px,
against 34x20 with a 14px knob travelling 14px). Same `--r-pill` track, `--line-strong` border,
`[data-on]` filling with `--accent`, knob turning `--accent-contrast`. mail adds a touch size
(46x28); editor does not.

The keycap: calendar palette.css:105-118 `.key` and mail ui/Key.css:1-17 `.key[data-size="md"]` are
byte-identical apart from mail moving the padding onto a size attribute. editor has a third,
`.key-cap` at tree.css:744, an `--accent-wash` chip with no border; margin a fourth, `.esc-hint kbd`
at app.css:2191.

Segmented control: mail ui/Segment.css:3-30 `.segment` / `.segment-option` and calendar app.css:483-508
`.view-switch` / `.view-option` are the same object (2px padding, 2px gap, `--r-md` track of
`--accent-wash`, `--r-sm` options, active option lifted onto `--paper`). Calendar has a third copy for
the phone at app.css:244-267 (`.tabbar-views` / `.tabbar-view`).

## Menus, popovers, row menus, toasts, resizers

- Dropdown menu. margin app.css:1382-1438 and editor app.css:510-588 share `.menu-wrap`,
  `.menu-backdrop`, `.menu`, `.menu button`, `.menu-label`, `.menu-sep`; five of those six bodies are
  byte-identical. `.menu` differs only in `min-width` (156 vs 168) and `.menu button` in editor
  gaining `grid-template-columns: 14px 1fr` for a glyph column. mail's equivalent, ui/Popover.css, is
  positioned from JS through `--pop-left` / `--pop-top` / `--pop-w` and is genuinely different.
- Row menu. `.row-menu-btn`, `.row-menu-btn:hover`, `.row-menu-pop`, `.row-menu-item`,
  `.row-menu-item:hover`, `.row-menu-item.danger`, `.row-menu-item.danger:hover` are byte-identical
  between margin app.css:393-460 and editor app.css:363-430. Seven rules, no differences.
- Toast. margin app.css:1476 and editor app.css:590 are byte-identical: fixed, `bottom: 26px`,
  centred by `translateX(-50%)`, `z-index: 40`, `max-width: 460px`, `--ink` ground with `--paper`
  text. calendar app.css:437 and mail ui/Toast.css:1 are a different and better toast, also nearly
  identical to each other: `bottom: 24px`, `z-index: 60`, `max-width: min(560px, calc(100vw - 48px))`,
  `--glass` with `blur(8px)`, a `--line` border and `--shadow-pop`. Two designs, two apps each.
- Resize handle. margin app.css:140-186 and editor tree.css:64-108 are the same rule set:
  zero-width flex item, an 8px `::before` hit area at `left: -4px`, a 2px `::after` accent line at
  `left: -1px` going to `opacity: 0.55` on hover. The only difference is the drag hook,
  `body.resizing` against `:root[data-resizing]`. editor/src/components/ResizeHandle.tsx writes
  `--pane-sidebar` on the root, so the token is already the interface.
- Selected-row accent edge. `.chapter[data-active="true"]::before` (margin app.css:275, editor
  app.css:266, byte-identical) and `.row[data-selected]::before` (mail ui/Row.css:29): an absolutely
  positioned 2-3px bar of `--accent` with `border-radius: 0 2px 2px 0`, inset from the row's ends.
- Sidebar and nav. `.sidebar`, `.brand`, `.brand .back-label`, `.brand:hover`, `.nav-label`,
  `.nav-scroll`, `.nav-section + .nav-section`, `.chapters`, `.chapter` and its nine state rules,
  `.chapter-drop`, `.add-chapter`: all byte-identical between margin and editor. The bulk of the 59
  identical bodies, and a straight copy of one app's sidebar into the other.
- Settings. editor styles/settings.css and mail screens/settings.css are the same layout (a rail
  left, a measured column right, rows of label plus control plus note) at different numbers: rail
  `var(--pane-sidebar)` (248px) against a literal 210px, column 620px against 640px,
  `.settings-nav-item` against `.settings-tab`, `.setting-row` against `.set-row`. Both gate the
  traffic lane with `:root[data-traffic] { padding-left: var(--traffic-pad) }`. margin and calendar
  keep settings inside `.panel`, which is a legitimate difference of kind.

## Themes

Four implementations, three of which are the same file.

margin/src/theme.ts, calendar/src/theme.ts and mail/src/theme.ts are the same 16 lines with one
string changed: the localStorage key (`margin-theme`, `margincal-theme`, `marginmail-theme`). Same
`initialTheme` reading `data-theme` off the root first, then storage, then
`matchMedia("(prefers-color-scheme: dark)")`; same `applyTheme` writing the attribute and the key.
The boot scripts in each index.html are the same shape too, differing in the key and in what else
they set on the root (calendar adds `data-phone`, `data-touch`, `data-view`; mail adds `data-phone`,
`data-touch`, `data-no-pane` and the two font slots).

editor/src/theme.ts is 167 lines and a different design: seven named palettes
(`light`, `sepia`, `mist`, `contrast`, `dark`, `graphite`, `midnight`), a `ThemeChoice` that can be
`"system"`, a remembered light half and dark half so "Match system" lands on the two the user
actually picks, storage in try/catch for a webview with storage denied, and `watchSystemScheme`
listening for the media query as an event. Its palettes live in editor/src/styles/themes.css, one
44-line block each, and editor/src/theme.test.ts reads that file and fails when a block is short.

No app uses `@media (prefers-color-scheme)` in CSS at all. All four resolve the system preference in
JS and write `data-theme` on the root. That is one decision, taken four times, and it is the right
one, so it should be taken once.

The multi-theme design is not a candidate for sharing as it stands: mail and calendar's stylesheets
have no idea `sepia` or `midnight` exist and would fall through to the light `:root` block. But the
three-line theme module and the boot script are, with the key as a parameter.

editor themes.css:307-322 is worth flagging: it copies six hexes of the shared light and dark
palettes so the picker's preview tiles can draw them, with a comment saying it is the only copied
colour in the file and that it is copied because "this repo may not reach into" margin-shared. It
can, and does, through `margin-shared/css/tokens.css`. A shared `.theme-swatch[data-theme="light"]`
block in the package would delete those two blocks.

## Drift: values hardcoded where a token exists

Counted over every CSS file except each app's token layer:

| app | hex literals | rgba() literals |
| --- | --- | --- |
| margin | 15 | 17 |
| calendar | 0 | 1 |
| editor | 0 | 0 |
| mail | 0 | 0 |

Editor and mail are clean. Calendar's one is `box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06)` at
app.css:507 on `.view-option[data-active]`, the only shadow in the app not from `--shadow-page` or
`--shadow-pop`. margin is where the drift lives, and most of it is a token it never adopted:

- app.css:1275 `background: rgba(35, 32, 27, 0.28)` on `.overlay`. That is `--scrim`, which the other
  three all have.
- app.css:1449 `background: rgba(35, 32, 27, 0.32)` on `.export-overlay`. A second scrim at a fourth
  of a percent difference, which nobody chose.
- app.css:272 and :1839 `box-shadow: 0 1px 2px rgba(35, 32, 27, 0.05)`. That is editor's
  `--shadow-raised` exactly.
- app.css:1547 `box-shadow: 0 4px 10px rgba(35, 32, 27, 0.1), 0 20px 38px rgba(35, 32, 27, 0.13)` on
  `.card:hover`. A third shadow that is neither `--shadow-page` nor `--shadow-pop`.
- app.css:2933 `background: rgba(0, 0, 0, 0.42)` on `.drawer-scrim`. A third scrim.
- app.css:1052 `0 1px 3px rgba(0, 0, 0, 0.3)`, app.css:1458 and :1465 `#fcfbf7` (`--paper`'s light
  value, hardcoded so it survives on the dark overlay), app.css:1464 `rgba(255, 255, 255, 0.28)`,
  app.css:638 `#fffefb`, app.css:678 `#2b2720`, app.css:1238 `#fff` (that last is editor's
  `--pdf-page`, which editor tokenised precisely because it was a literal in two stylesheets).
- app.css:2184 `border-radius: 999px`, which is `--r-pill` in the other three.
- app.css:75 `padding: 0 14px 0 84px` on `.titlebar`. The 84px is the macOS traffic-light lane,
  applied on every platform with no `data-traffic` gate. calendar app.css:126, editor app.css:97 and
  mail header.css:24 all gate it, and the comment at calendar app.css:121-125 says what the ungated
  version costs on Linux, Windows and iPad.
- app.css:2881 `--titlebar-h: calc(46px + env(safe-area-inset-top))` inside `.app[data-compact]`.
  It restates the 46px shared already owns and reads `env()` inline where calendar and mail both have
  a `--safe-top` token for it.

The twelve proofing colours at app.css:2367-2451 (`#b4453a`, `#9c6e16`, `#2f6e4f` and their dark
counterparts, plus six washes) are a real palette with no token, and editor has the same feature
(styles/proofing.css) using `--danger` and friends. Worth comparing separately; it is the one place
margin's literals encode a design rather than a forgotten token.

## What one shared stylesheet would have to contain

Tokens, added to shared/css/tokens.css: `--scrim`, `--shadow-raised`, `--r-pill`, `--t-5`,
`--touch-h`, `--traffic-pad`, `--safe-top`, `--safe-bottom`, `--phonebar-h`, `--tabbar-h`,
`--sheet-max-h`, and the eight-hue ramp under one name. Plus the `--ink-faint` correction. That is
19 names and one fix, and it removes every one of them from calendar, editor and mail's own layers.

A base sheet: `*`, `html, body, #root`, `html` (with the tap-highlight line), `body`, `::selection`,
`:focus-visible`, `button` (with `font-size: inherit`), `input, textarea, select`, `svg`, `.app`, the
`data-touch` selection rules and the `data-phone` field-size rule. All of it is already identical or
one line from identical in all four.

An overlay sheet: `.overlay`, `.overlay[data-align="top"]`, `.panel`, `.panel-head`, `.panel-head h2`,
`.panel-body`, `.panel-foot`, the four `:root[data-phone]` docking rules, `@keyframes sheet-up` and
its reduced-motion guard. Byte-identical or trivially reconcilable across all four today.

A controls sheet: the button (mail's `[data-variant]` and `[data-size]` version, which is the
superset), the field, the label, the toggle, the keycap, the segmented control, the icon button under
one name, the row menu, the toast (calendar and mail's version), the resize handle. Every one of
these exists in at least two apps already and differs by a padding value or a class name.

`color-scheme` at the root, per theme, which only editor has, and a `prefers-reduced-motion` guard
convention, which only mail applies consistently.

What each app keeps: editor keeps its document scale, sheet padding, code surface and its seven-palette
themes.css; mail keeps its mail geometry, the `--message-*` set that deliberately ignores the theme,
and its `--check` OS blue; calendar keeps its grid geometry and the `--grid-*`, `--fold-*` and
`--event-*` palettes; margin keeps `--pane-dock`, its scrollbar rule (or that moves up), its proofing
palette and the device-frame `--dv-*` set. Nothing else in the four apps' CSS is app-specific by need
rather than by accident.
