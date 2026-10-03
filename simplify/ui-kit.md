# UI kit

The plan for `@margin/ui`, the React primitives. Hooks and the IPC wrapper are [hooks.md](hooks.md);
tokens and stylesheets are [design-system.md](design-system.md).

## The kit already exists

Margin Mail's `src/ui` is seventeen primitives behind one barrel, with `screens/Kit.tsx` (613 lines)
rendering every one in every state in both palettes. It was built as a component library and it
behaves like one. The other three apps each hold a partial, earlier, differently named copy of about
two thirds of it.

So the work is not designing a component library. It is promoting `margin-mail/src/ui` into
`@margin/ui`, reconciling three class vocabularies against it, and deleting the rest. That framing
matters, because it turns a design exercise into a mechanical one with a reference implementation and
a page that proves it renders.

Sizes for scale: components are 3,263 lines in Margin, 6,063 in Margin Calendar, 4,938 in Margin
Docs, and 12,003 across Margin Mail's `ui` and `screens`. Roughly 1,600 lines of TSX and 900 of CSS
collapse to about 900 and 500.

## Two things block this before any code moves

**A standing decision says no.** `shared/src/icons.ts:11-13` states it in the file:

> Each app renders these through its own `Icon` component. The two components are identical today
> and are deliberately not shared: one is React, which would make this package depend on React for
> twenty four lines, and a component is where an app is entitled to differ.

That was reasonable when the surface was 24 lines. It is not now: `margin/src/components/Icon.tsx`,
`margin-caledar/src/components/Icon.tsx` and `margin-editor/src/components/Icon.tsx` are byte
identical (md5 `0ec1a568818f20ed8eed8ad46fbaa2b1`), Margin Mail's `ui/Icon.tsx` adds a class and
`aria-hidden`, and beneath them sit 1,600 lines of duplicated component code. The React dependency
argument is also mechanically wrong: `shared/package.json` has no dependencies block, all four apps
are on `react ^19.1.0`, and a peer dependency costs nothing.

Reopen that note explicitly, in the file, with the new reasoning. Do not quietly contradict it.

The mechanical consequence: the package is source only with no build step, so each app's tsconfig and
Vite config has to compile TSX out of `node_modules`, and none does today. That is a
[toolchain.md](toolchain.md) change and it gates everything here.

**Margin Calendar is not in the package.** `grep -r margin-shared` over its tree returns nothing. It
carries its own 168 line `tokens.css` against the shared 91 line one. It also has the most to gain,
because Margin Mail already forked two of its components. One dependency line is the prerequisite for
every item below.

## Icon, and the alignment problem

The alignment problem the same fix keeps being applied to is not sub-pixel. There is not one use of
`shape-rendering`, `vector-effect`, `crispEdges` or a half-pixel translate anywhere in the four repos.
It is two ordinary CSS facts: an inline SVG sits on the text baseline, and it shrinks as a flex item.

Four apps fix that five different ways. Margin Mail alone fixes both centrally, with
`svg { display: block }` at `app.css:88` and `.icon { flex: none }` in `Icon.css`. The other three
patch per call site with `transform: translateY(2px)`, `margin-top: 2px` and `align-self: center`.

`@margin/ui` ships Margin Mail's `Icon` and Margin Mail's two rules. Every per-site nudge in the other
three comes out. This is the single change that stops the icon alignment tax, and it is four lines.

Glyph paths stay in `@margin/icons` as bare strings. Margin Mail's `ui/icons.ts` shows the right
pattern for app-specific glyphs: 29 paths, five of which re-export from the shared set rather than
redeclaring them. Where paths have drifted, the shared set takes the corrected one. The clearest case
is documented in the code: `margin-editor/src/components/Toolbar.tsx:182` explains that the H moved
from x=5 to x=7 because it "sat left of centre in a round button", and Margin still has the
uncentred version. Same for the bullet list glyph, x=3.5 in Margin against x=4 in Margin Docs.

## The order of work

Ranked by payoff against the amount of argument required, not by line count.

| Rank | Cluster | Apps | Now | After | Argument needed |
|---|---|---|---|---|---|
| 1 | `Sheet` and `Confirm` plus panel CSS | 4 | ~300 tsx, ~240 css | ~170, ~60 | none, Mail already forked Calendar's |
| 2 | `useRovingFocus`, `stepIndex`, scroll hook | 4 | ~200 over 20 sites | ~60 | wrap or clamp must be settled |
| 3 | PDF export preview | 2 | 784 | ~450 | none |
| 4 | Button, Toggle, Segment, Key, Field, Icon, EmptyState, Pill, GroupHead | 4 | ~660 tsx, ~600 css | ~410, ~350 | one class vocabulary wins |
| 5 | Find bar | 2 | 475 tsx, 370 css | ~320, 161 | none |
| 6 | Shortcuts sheet | 3 | ~170 tsx, ~130 css | ~60, ~50 | Docs adopts the sheet |
| 7 | Palette shell, `commandMatches`, `highlight` | 3 | ~510 | ~250 | wrap versus clamp, row identity |
| 8 | Menu and anchored positioning | 4 | ~890 | ~450 | six behaviours to reconcile |
| 9 | Toast | 4 | ~140 | ~60 | two visual designs |
| 10 | `Stage`, `RecentList`, `ProgressBar`, `OAuthPending` | 4 | ~250 | ~130 | none |
| 11 | `ResizeHandle` and pane width | 2 | 177 | ~110 | none, and three bugs fixed |
| 12 | `SearchHighlight` and `positions.ts` | 2 | ~590 | ~300 | none |

Settings is deliberately absent. See the end of this document.

## Sheets, dialogs and confirmation

Every app's answer to a floating panel over the app is already the same idiom: a flat list of self
mounting overlay components at the end of `App.tsx`, each reading its own store and returning `null`
when closed. Underneath, `src/escape.ts` is byte identical in all four.

Margin Mail's `ui/Sheet.tsx` is Margin Calendar's `components/overlayShell.tsx`, forked, with the
comments carried over verbatim. The differences are Mail's improvements: `onBack` and `backLabel` as
props rather than reading a store and a hardcoded titles map, with the reason given at
`Sheet.tsx:33-36` ("a primitive that imports one cannot be rendered on a Kit page"), and `busy`, which
makes the close control, the scrim and Escape all refuse while a command is in flight. That `busy`
prop is what makes [guidelines/errors-and-feedback.md](guidelines/errors-and-feedback.md) enforceable
rather than aspirational, so it belongs in the primitive.

`ConfirmDialog` in Margin and Margin Docs each has half the correct behaviour. Margin calls
`useFocusTrap` and Docs does not; Docs sets `role="dialog"` and `aria-modal` and Margin does not. The
class names drift by one letter, `icon-btn` against `icon-button`.

One behaviour has to be settled rather than merged: Margin and Margin Docs focus the destructive
button in a confirmation, Margin Calendar and Margin Mail focus cancel and say why in a comment ("a
stray Enter does nothing destructive"). Cancel wins.

The CSS is one design in four copies. Across the four `app.css` files, `.panel` and `.panel-body` have
exactly one distinct body between them, `.overlay` has two (Margin hardcodes `rgba(35,32,27,0.28)` at
`app.css:1275` where the rest use `var(--scrim)`), `.panel-foot` two, and `.panel-head` three,
differing by 2px of padding and a `flex: none`.

Ship `Sheet` and `Confirm` as Mail declares them plus `panel.css`. `ConfirmDialog` becomes
`<Sheet size="mini"><Confirm/></Sheet>`. Keep `ConflictDialog`'s reload and keep semantics and
`MoveChapterDialog`.

## List navigation, which is the largest copied thing in the audit

There are five distinct expressions of "move the selection by one" across the four apps, and the split
is not by app: Margin Docs alone contains three of the five. Clamp with a seed from whichever end the
delta came from, four near-identical copies. Clamp with no seed, where two Docs files are literally
the same line and one says so in a comment. Clamp by indexing off the end and guarding `undefined`.
True modulo wrap, in seven places. Wrap plus seed, in two.

The result is that a list wraps or does not depending on which app and which surface you are in, which
is precisely what a shared design language is supposed to settle.

```ts
useRovingFocus(ref, { selector | refs, wrap, homeEnd, seedFromEnd, onMove })
stepIndex(count, at, delta, { wrap }): number | null
useScrollSelectedIntoView(scrollerRef, selectedId, { attr: "data-id" })
```

`useRovingFocus` has ten call sites, eight of which are the same eight lines differing only in the CSS
class queried and whether they wrap. `stepIndex` replaces seven copies.

The scroll hook must use Margin Docs' manual arithmetic, not `scrollIntoView`.
`margin-editor/src/components/Outline.tsx:80-90` deliberately avoids `scrollIntoView` because, as the
comment at `:79` says, it "is free to scroll every ancestor of the row as well". Four other places
still use it and still have that bug.

Two constraints on the API. It must take key predicates rather than hardcoding `e.key`, because Mail
and Docs route list keys through a remappable binding table while every menu listens for a literal
`ArrowDown`. And it must take a scroll strategy rather than assuming the DOM, because Mail's
`ListColumn` drives a react-virtuoso handle.

Two gaps worth closing while this is open: there is no type-ahead anywhere in any app, and Home and
End exist in only three places. `margin/src/components/Library.tsx` is the one screen that gains a
feature rather than loses duplication, since its card grid has no arrow navigation at all.

## The primitives

Margin Mail has the component in every case below; the others have the markup.

**Button.** Mail's `ui/Button.tsx` with `default | primary | ghost | danger`, 21 uses in its Settings
alone. Calendar expresses the same vocabulary as a CSS attribute, `.panel-button[data-variant]`;
Margin uses `.btn-primary`, `.btn-ghost`, `.btn-danger`; Docs adds `.btn-quiet`. Four conventions, one
control. Mail's names win because they are already a component API rather than a class convention.

**Icon button.** Eleven independent square-icon-button rules across the four apps, all `place-items:
center` with `--r-sm` and an `--accent-wash` hover, differing only in size (16, 22, 26, 28, 30) and in
whether they are called `.icon-btn` or `.icon-button`. There are 54 hand-written call sites in the
three older apps. Mail's is the only component and the only one that supplies `aria-label`
automatically, which is why the other three have unlabelled icon buttons.

**Toggle.** Mail's `ui/Toggle.tsx` (46 lines plus 78 of CSS), nine uses. Docs inlines the same
`<button role="switch" aria-checked data-on>` with a knob on `translateX` inside its settings row.
Margin uses a raw checkbox, Calendar has none.

**Segment.** Mail's `ui/Segment.tsx`, ten uses, against Calendar's two inline copies. Same class
names, opposite visual models: Calendar paints the active option `--accent`, Mail lifts it onto
`--paper` in an `--accent-wash` track. Mail has `role="tablist"`, Calendar has no ARIA. Mail's model
wins on both counts.

**Field, Key, EmptyState, Pill, GroupHead.** Mail has all five. Note that Mail's own Settings uses
`ui/Field.tsx` inconsistently, with a local commit-on-blur draft and three raw inputs beside it; the
extraction is the moment to settle that rather than carry it across.

**Select.** Nobody abstracted it. It is written five times across two apps, all
`<select className="settings-select">` with the same "unknown current value gets its own leading
option" hatch. Mail's font picker and Margin's `FontSelect` are near duplicates and both already
driven by the shared font catalogue, so they collapse first.

Settle one class vocabulary at the same time. Docs and Calendar are one word apart on the settings row
(`.setting-label` against `.setting-name`), and Mail's is the only one that names the control slot and
takes `children` rather than baking a switch into the row. Mail's wins.

## Menus and anchored positioning

Six implementations of one object, and no two agree on the table below.

| | positioning | edge clamp | portal | dismiss | Esc | trap | restores focus |
|---|---|---|---|---|---|---|---|
| margin `RowMenu` (161) | anchor rect | none | yes | mousedown capture | yes | yes | yes |
| margin `Menu` (42) | CSS only | none | no | backdrop div | no | yes | via teardown |
| margin `AddPageMenu` (105) | anchor rect | none | no | mousedown | yes | yes | yes |
| docs `RowMenu` (187) | point | both axes | yes | mousedown capture | yes | no | yes |
| docs `WidthMenu` (187) | CSS only | none | no | backdrop and blur | yes | no | keyboard only |
| mail `Popover` (108) | anchor rect | x only | no | pointerdown capture | yes | no | no |

Two of these are bugs rather than differences. `margin/src/components/RowMenu.tsx:33-37` sets
`top: r.bottom + 4` with no clamping, so a row low in a long chapter list opens a menu off the bottom
of the window. `margin/src/components/Menu.tsx` has no escape layer at all, so Escape does not close
it.

The best placement maths in the suite is not in a menu. It is
`margin-caledar/src/components/EventDetailsModel.ts:71-104`, a pure, unit-tested function that tries
right, left, below, above, then centre, clamps the cross axis and returns the side it chose. That
becomes `useAnchoredPosition`.

Two ideas only one app has, both worth keeping. Docs' caret bargain, where `e.detail === 0` detects
keyboard activation and only then moves focus, with mouse presses `preventDefault`ed so the caret
stays in the sentence. And Mail's reposition-rather-than-close on scroll and resize, which is right
for a contact card in a scrolling thread and wrong for a menu.

Share the placement hook and the menu body. Do not share the dismissal policy: `Popover` stays a
separate primitive from `Menu` because they answer different questions.

## Palette

Margin has none. Docs has a shell with three consumers, Mail a shell with one, Calendar no shell and
one monolith.

The command matcher is one function copy-pasted three times, character for character, down to the
variable names `needle`, `hay` and `at`. It lifts verbatim as `commandMatches`.

The keyboard model diverges in ways a user would notice moving between the apps: lists wrap in Docs
and Calendar and clamp in Mail; Ctrl+N, Ctrl+P and Tab move the selection in Calendar only;
`scrollIntoView` on the selection is Docs only; `aria-activedescendant` and `role="combobox"` are Docs
only, so Mail and Calendar announce nothing when the arrows move; group headers are Mail only; match
highlighting is Docs only.

No app has all of these, which is the strongest single argument in this document: consolidating is a
strict upgrade for every consumer rather than a wash.

```ts
interface PaletteItem { id: string; run?: () => void }
interface PaletteSection<T extends PaletteItem> { id: string; label?: string; items: readonly T[] }

<Palette label placeholder query onQuery sections status renderItem onChoose onClose
         wrap = true          // Mail's clamp becomes opt-out
         extraKeys = true     // ctrl+n/p and Tab, Calendar's model
         header />            // Calendar's parse preview block
```

Fix while it is open: `margin-mail/src/screens/CommandPalette.tsx:158-173` registers a window keydown
listener with no dependency array, detaching and reattaching every render. The comment at `:156` says
this keeps the closure fresh; Docs gets that for free by handling on the input instead.

The CSS is already shared in fact. Mail's `ui/Palette.css` and Calendar's `styles/palette.css` have
the same `width: min(620px, calc(100vw - 32px))` and `max-height: min(560px, 76vh)` and identical row,
input, list and keys rules. Docs diverges and adopts.

Keep the content matchers out. They are three different problems: a Rust fzy scorer, SQLite FTS5
`bm25`, all-terms substring over three concatenated fields, and a backend parse.

## The PDF export preview

The largest single-file duplicate in the tree, and the cleanest win with no design argument attached.
`margin/src/components/ExportPreview.tsx` (336) against
`margin-editor/src/components/ExportPreview.tsx` (448). Docs says so at `:3-5`: "The sibling book app
answers Export with this same panel, and both apps answer it this way for the same reason."

It is the same code, not the same idea. The `Frame` interface and the pane measurement, the toolbar
with the same glyphs and the same `ZOOM_MIN` and `ZOOM_STEP`, a primary button whose label is the same
ternary (`saving ? "Saving…" : compact ? "Save" : "Save PDF…"`), the fit arithmetic character for
character (`Math.max(240, Math.min(stage.width - 56, (stage.height - 56) / ratio))`), and the lazy
page renderer down to the 1400px `rootMargin`, the device pixel ratio clamp of 2 and the
`task?.cancel()` teardown.

About 200 of Margin's 336 lines and 220 of Docs' 448 are one component. App-specific are the compile
call, the save path and the warning text.

    <PdfPreview bytes title onSave saving warning onClose />

## Toast, find bar, shortcuts sheet, empty stages

**Toast.** Margin has no component at all: the same six lines of markup and the same timer effect
appear twice, in `EditorView.tsx` and `Library.tsx`. Calendar's and Docs' components differ by a
constant name and a `title="Dismiss"`, and their `useToast.ts` files are byte identical. Mail splits
presentation from the store binding and adds what the others lack: an action button with a keycap, and
a `seq` counter so an identical message twice restarts the timer where the other three do nothing on a
repeat. Dwell times are 4000, 5000, 4200 and 6000; pick one. Two visual designs exist, glass in
Calendar and Mail, inverted `--ink` on `--paper` in Margin and Docs; that one needs a decision.

**Find bar.** Margin and Docs only. The render block is the same component, and diffing the CSS gives
three real changes in 161 lines. The difference is ownership: Docs declares a `DocumentFind` interface
and takes it as a prop, explaining that a bar drawing a text field has no business owning a ProseMirror
decoration set, while Margin imports the book store and the chapter model directly and carries 90
lines of cross-chapter scope logic. Docs' shape wins, plus a `scope?: {label, onToggle}` for Margin.

**Shortcuts sheet.** Calendar's and Mail's open with the identical three-line comment and close with a
note whose first sentence is word for word "Nothing is modal and nothing is chorded." Docs' is the
earlier form with its own vocabulary. Margin has no sheet and no binding table to generate one from,
so it gains this only when it gains a binding table.

**Empty stages.** Four apps, four class vocabularies, one layout: a mark, an `h1`, one line of prose,
a row of buttons, one line of fine print. Ship `<Stage>`, `<RecentList>`, `<ProgressBar>` (five copies
today, four of them inside Mail) and `<EmptyState>`.

**The OAuth pending block** is the strongest single duplication in this group.
`margin-caledar/src/components/Accounts.tsx:88-114` and `margin-mail/src/screens/Connect.tsx:136-171`
are the same block: a "waiting in your browser" line, a note conditional on `authUrl`, and Open link,
Copy link and Cancel wired to the same three store actions with the same phase names, both guarding
Escape with an identical `useEscapeLayer(phase === "connecting", cancelConnect)`. It becomes
`<OAuthPending ready onOpen onCopy onCancel>` and pairs with the crate in [accounts.md](accounts.md).

## Pane resize

Two apps, not four: Calendar has no sidebar and no resizable pane, and Mail's `--list-w` is a
constant. Margin's `ResizeHandle.tsx` plus `panes.ts` against Docs' `ResizeHandle.tsx`. The drag body
is the same algorithm line for line, with the same MIN 200, MAX 460 and DEFAULT 248, and the CSS is
near verbatim.

Each has half the correct behaviour, and both are missing three things:

- Margin has keyboard resize with a 16px step and Home to reset, an `aria-label` and `tabIndex={0}`.
  Docs' separator cannot be focused at all.
- Docs wraps storage in try and catch. `margin/src/panes.ts:41` throws on a webview that denies
  localStorage.
- Neither handles `pointercancel` or calls `releasePointerCapture`, so a cancelled pointer leaves the
  listeners attached and `cursor: col-resize` pinned on the document.
- Neither debounces. A 120Hz drag issues 120 synchronous `localStorage.setItem` calls per second, with
  no rAF anywhere.
- Docs flashes 248px and jumps on boot, because its boot script restores theme, sidebar and width but
  not the pane width, leaving that to a `useLayoutEffect`.

## Per app

**Margin Calendar** first, because it is not in the package and everything else depends on that. Add
the dependency, adopt the tokens, then take `Sheet` and `Confirm` back in the form Mail forked them
into, then the primitives.

**Margin Mail** second, and its work is mostly outward: move `src/ui` into the package, keep `Kit.tsx`
in the repo as the app's own proof page, and re-import. It also settles the class vocabulary, since
its names win almost everywhere.

**Margin Docs** third: the palette shell, the find bar, the row menu body, the resize handle, and the
export preview shared with Margin. It gives up its settings row vocabulary and its shortcuts sheet
form.

**Margin** last and largest, because it has the most hand-rolled markup and the least structure: no
toast component, no palette, no binding table, no first-run screen, and the two worst menu bugs. Its
export preview is the one item it can do early and independently.

## What deliberately stays

**The Settings shell.** Generic chrome is 9% of Mail's 2,551 line Settings, 22% of Docs' 346, 16% of
Margin's 215, and 0% of Calendar's because its chrome is already in `Sheet`. Under 200 lines saved out
of 3,402, and a `SettingsShell` would have two consumers who disagree about a header, a close button
and a drag region. Ship the row primitives, leave the shell. Mail's keyboard section navigation does
not port either, because it re-points the app's own `j` and `k` at the rail to get it.

**Any `<Editor>` or shared tiptap extension list.** Three content types, three schema policies, three
lifecycles, and `margin-editor/src/editor/extensions.ts:1-20` is a written argument against the list
specifically. Extractable instead: `SearchHighlight` and `searchStateOf` (a 73-line diff across 233
and 264 lines, with Docs' header saying the only change of substance is a rename), `positions.ts`, the
toolbar primitives, and the install-then-restore-position helper with its `document.fonts.ready` pass.
A small editor kit, not an editor.

**Spinners and loading states.** Four different product positions, not four copies of one. Mail bans
spinners on the record. Calendar has none.

**Setup flows.** Exactly one component in four apps has numbered steps and it is a slideshow.
`Connect.tsx` and `ConnectMail.tsx` look like wizards and are not: their states are phases of an
external process the user cannot navigate. Four screens appearing at the same moment in a product's
life, sharing an aesthetic, not a shape.

**Things that only look alike.** Calendar's `ColorPicker` is a Google `colorId` radio group and Mail's
`Avatar` is a hashed-hue initials badge. `ProofPopover` is the same feature with the same classes, but
Docs has grown a keyboard walk, an escape layer, a focus-return policy and a flip-above fallback that
Margin has not; share the anchored-menu primitive underneath and leave the issue rendering in each app.
And `src/width.ts` shares a filename across Margin and Docs while meaning unrelated things: rename one
rather than reconcile them.
