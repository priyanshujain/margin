# Duplicated React components across the four apps

Components only. Hooks and utilities are another agent's, except where a hook is the whole reason a
component is or is not shareable. Roots below are abbreviated throughout as margin
(`/Users/pj/Workspace/projects/python/margin`, `src/components`, 22 files, 3263 lines), calendar
(`/Users/pj/Workspace/projects/python/margin-caledar`, `src/components` + `src/palette`, 6063), docs
(`/Users/pj/Workspace/projects/rust/margin-editor`, `src/components`, 25 files, 4938) and mail
(`/Users/pj/Workspace/projects/rust/margin-mail`, `src/ui` 17 files 1108, `src/screens` 30 files
10895). Totals: tsx is 3826 / 5758 / 7350 / 12684, CSS 2983 / 3796 / 5089 / 6571.

## The one-line answer

margin-mail already built the shared package. `mail/src/ui` is seventeen primitives behind one barrel
(`ui/index.ts`), with a Kit page (`screens/Kit.tsx`, 613 lines) rendering every one in every state in
both palettes. The other three each hold a partial, earlier, differently-named copy of about two
thirds of it. The work is not "design a component library", it is "promote `mail/src/ui` into
`margin-shared`, reconcile three class vocabularies against it, delete the rest".

## Two things that block this before any code moves

**1. A standing decision says no.** `shared/src/icons.ts:11-13`, in the file itself:

> Each app renders these through its own `Icon` component. The two components are identical today
> and are deliberately not shared: one is React, which would make this package depend on React for
> twenty four lines, and a component is where an app is entitled to differ.

Reasonable when the surface was 24 lines. `margin/src/components/Icon.tsx:1-24`,
`calendar/src/components/Icon.tsx:1-24` and `docs/src/components/Icon.tsx:1-24` are byte-identical;
mail's (`ui/Icon.tsx:1-29`) adds a class and `aria-hidden`. Below them sit roughly 1600 lines of
duplicated component code and 900 of duplicated CSS. That note has to be reopened explicitly.
Mechanically the package has no build step (`shared/package.json:8-16`, source-only exports), so each
app's tsconfig and Vite config must compile TSX out of `node_modules`, and none does.

**2. Calendar is not in the package.** `grep -r margin-shared` over the calendar tree returns nothing;
it carries its own 168-line `src/styles/tokens.css` against the shared 91-line one. And calendar would
gain most, because mail already forked two of its components.

---

## Sheets, dialogs, confirmation

First, the encouraging part. Every app's answer to "a floating panel over the app" is the same idiom:
a flat list of self-mounting overlay components at the end of `App.tsx`, each reading its own store
and returning `null` when closed (`margin/App.tsx:105-107`, `calendar:122-135`, `docs:221-238`,
`mail:382-432`), and underneath, `src/escape.ts` is **byte-identical in all four**. A shared overlay
component composes in all four on day one, provided it takes props rather than reading a store.
`useFocusTrap` is margin-only (`src/focus.ts`, 56) and used by seven of its components; share it for
dialogs and sheets, not menus, since `docs/WidthMenu.tsx:129-136` wants tab-out to work.

margin and docs write `.overlay`/`.panel` inline per dialog (`ConfirmDialog` 48 and 56,
`ConflictDialog` 66); calendar has `components/overlayShell.tsx` (137) and mail `ui/Sheet.tsx` (162),
each with a `Confirm` in the same file.

**mail's `Sheet.tsx` is calendar's `overlayShell.tsx`, forked.** Same props, same DOM, comments
verbatim: `overlayShell.tsx:4-6` and `Sheet.tsx:30-31` are both "Nothing is resident, so a closed
sheet renders nothing at all and its children mount fresh on the next open"; `overlayShell.tsx:50` and
`Sheet.tsx:55` both "Focus has to leave the grid/page or the first keystroke goes to the keymap
instead of the panel". The diffs are mail improvements: `onBack`/`backLabel` as props
(`Sheet.tsx:15-17`) rather than reading `useOverlays` and a hardcoded `TITLES` map
(`overlayShell.tsx:27-43`), the reason given at `Sheet.tsx:33-36` ("a primitive that imports one
cannot be rendered on a Kit page"); and `busy`, which makes the close control, the scrim and Escape
all refuse while a command is in flight (`:23`, `:52`, `:69`, `:102`).

`ConfirmDialog` in margin and docs each has half the correct behaviour: margin calls `useFocusTrap`
(`:23`) and docs does not; docs sets `role="dialog" aria-modal` (`:32-33`) and margin does not. Class
drift: `icon-btn` (`margin:30`) vs `icon-button` (`docs:38`). Button order is consistent everywhere
(cancel left, destructive right) but **margin and docs focus the destructive button** (`margin:19`,
`docs:24`); calendar and mail focus cancel and say why (`overlayShell.tsx:114`, `Sheet.tsx:143`: "a
stray Enter does nothing destructive").

The CSS is one design in four copies: across the four `app.css` files `.panel` and `.panel-body` have
exactly one distinct body, `.overlay` two (margin hardcodes `rgba(35,32,27,0.28)` at
`margin/app.css:1275`, the rest use `var(--scrim)`), `.panel-foot` two, and `.panel-head` plus
`.panel-head h2` three, differing by 2px of padding and a `flex: none`.

Shared: `Sheet` and `Confirm` as mail declares them (`ui/Sheet.tsx:7-25`, `:113-122`) plus `panel.css`;
`ConfirmDialog` becomes `<Sheet size="mini"><Confirm/></Sheet>`. Stays: `ConflictDialog`'s reload/keep
semantics (`docs:12-21`), `MoveChapterDialog` (87). ~300 tsx to ~170.

**The update dialog** is the same story one level up: four answers, four phase unions. margin
`UpdateDialog.tsx` (88) is `checking|downloading|installing|uptodate|error`; docs (147) is
`available|downloading|installing|error` with release notes, `bytes()` (`:21-25`),
`role="progressbar"` with `aria-valuenow` (`:96-99`) and a Later button; calendar has **no dialog**
and says so at `src/keys/updates.ts:1-3` ("Ported from margin's `src/updater.ts`, minus its progress
dialog: there is no update UI here yet, so the toast carries the whole story"); mail has a Settings
row (`Settings.tsx:2466-2540`), `idle|checking|current|found|installing`. Docs' is the only one
showing notes, with a real progressbar role, that lets you decline, and its header comment (`:1-11`)
is the design rationale for all four. A shared `<UpdateDialog>` is worth doing, but it is a behaviour
decision first and a component second.

## Command palette, quick open, search overlay

margin has none: no palette, no quick open, no `src/keys`. Docs has a shell (`Palette.tsx`, 191) with
three consumers (`CommandPalette` 78, `QuickOpen` 166, `FindInFiles` 128); mail a shell
(`ui/Palette.tsx`, 127) with one (`screens/CommandPalette.tsx`, 191); calendar no shell, one monolith
(`palette/CommandPalette.tsx` 190 + `parse.ts` 249).

**The command matcher is one function copy-pasted three times, character for character**, down to the
names `needle`, `hay`, `at`: `docs/src/keys/commands.ts:422-433`, `mail/src/keys/commands.ts:64-75`,
`calendar/src/keys/commands.ts:213-224`. Character subsequence, case-insensitive, whitespace stripped
from the query, boolean not a score, and all three render in registry order with no ranking. The
content matchers by contrast are three different problems and should not be shared: docs' fzy scorer
is Rust (`margin-editor/src-tauri/src/index.rs:1010-1088`, weights at `:990-1003`), find-in-files is
FTS5 `bm25` (`index.rs:1187`), calendar's is all-terms substring over three concatenated fields
(`AgendaModel.tsx:262-273`), mail's is parsed in Rust (`SearchBar.tsx:14-18`).

Keyboard divergence a user would notice moving between apps:

- Wrap at the list ends: modular in docs (`Palette.tsx:91-92`) and calendar (`usePalette.ts:21-26`),
  **clamped in mail** (`CommandPalette.tsx:165`).
- Ctrl+N/Ctrl+P and Tab move the selection: calendar only (`CommandPalette.tsx:100-108`).
- `scrollIntoView` on the selection: docs only (`Palette.tsx:75-77`).
- `aria-activedescendant` and `role="combobox"`: docs only (`Palette.tsx:119-123`). Mail and calendar
  announce nothing when the arrows move.
- Group headers: mail only (`ui/Palette.tsx:16-20`, `:88-91`).
- Match highlighting: docs only (`Palette.tsx:169-191`), fed ranges from Rust rather than recomputed.
  Calendar has the same idea for search results as `splitMatch` (`AgendaModel.tsx:281-303`).

**No app has all of these.** That is the strongest argument in the audit: consolidating is a strict
upgrade for every consumer, not a wash. Row identity differs too: numeric index in docs and calendar,
string id in mail, which mail dispatches by parsing prefixes off (`CommandPalette.tsx:143-151`), while
docs puts a `run` closure on the row (`Palette.tsx:19-24`), which is why its shell is generic over
three unrelated data sources. One bug worth fixing while it is open:
`mail/screens/CommandPalette.tsx:158-173` registers a `window` keydown listener with **no dependency
array**, detaching and reattaching every render; the comment at `:156` says this keeps the closure
fresh, but docs gets that free by handling on the input (`Palette.tsx:86-99`). The CSS meanwhile is
shared in fact: `mail/ui/Palette.css` (99) and `calendar/styles/palette.css` (176) have the same
`width: min(620px, calc(100vw - 32px))`, `max-height: min(560px, 76vh)`, and identical
`.palette-input`, `.palette-list`, `.palette-row`, `.palette-keys`. Docs diverges.

```
interface PaletteItem { id: string; run?: () => void }
interface PaletteSection<T extends PaletteItem> { id: string; label?: string; items: readonly T[] }

<Palette label placeholder query onQuery sections status renderItem onChoose onClose
         wrap = true          // mail's clamp becomes opt-out
         extraKeys = true     // ctrl+n/p and Tab, calendar's model
         header />            // calendar's parse preview block
```

`status` is docs' `{text, error?}` machine, into which mail's single string collapses. Plus
`commandMatches` (lifts verbatim) and `highlight(text, ranges)` with a `rangesFromTerms` helper for
calendar's term form. Stays: the fzy scorer, the FTS path, calendar's `parse.ts`/`create.ts`, mail's
id-prefix dispatch and group assembly, every app's status copy. About 250 to 300 lines.

## Find bar

margin (285), docs (190), plus docs' `FindInFiles.tsx` (128). Calendar and mail have neither.

The render block is the same component. `margin:212-283` against `docs:103-188`: same
`.find-bar > .find-expand + .find-stack > .find-row`, same chevron paths `M6 9l6 6 6-6` /
`M9 6l6 6-6 6`, same `Aa` and `ab` toggles with `data-on`, same prev/next glyphs at `size={14}`, same
"No results" / "3 of 12" label, same Enter and Shift+Enter. Diffing `margin/app.css:2142-2350` against
`docs/tree.css:498-658` gives three real changes in 161 lines.

The difference is ownership. Docs declares a `DocumentFind` interface (`:34-44`) and takes it as a
prop, with `:5-9` explaining that a bar drawing a text field has no business owning a ProseMirror
decoration set; margin imports `buildRegex`, `getSearchState`, `useBook` and the chapter model
directly (`:2-7`) and carries ~90 lines of cross-chapter scope logic (`:122-210`). Shared: docs'
`<FindBar find={DocumentFind|null} />` plus `scope?: {label, onToggle}` for margin, about 130 tsx and
161 CSS to one copy. `FindInFiles` is a third consumer of docs' `Palette`.

## Toast

margin has no component: the same six lines of markup and the same timer effect appear **twice**, at
`EditorView.tsx:213-217` + `:505-509` and `Library.tsx:54-58` + `:161-165`. Calendar's `Toast.tsx` (24)
and docs' (23) differ by a constant name and a `title="Dismiss"`, and their `useToast.ts` (15 each) are
**byte-identical**. Mail splits presentation (`ui/Toast.tsx`, 29) from the store binding
(`screens/Toasts.tsx`, 47) and adds what the others lack: an action button with a keycap
(`ui/Toast.tsx:8`, `:21-26`) and a `seq` counter so an identical message twice restarts the timer
(`store/useToast.ts:18`, `:29`), where the other three do nothing on a repeat. Dwells are 4000, 5000,
4200, 6000. `.toast` CSS is two designs, two apps each: glass (calendar, mail) and inverted
`--ink`-on-`--paper` (margin, docs, byte-identical). ~140 lines to ~60.

## Settings

margin 215 + `BackupSettings.tsx` 179; calendar 111; docs 346 + 448 css; mail 2551 + 593 css.

Three shapes. **Mail and docs agree on the shell**: full window, a left nav of section names, a right
column of rows at a 620-640px measure (`mail/settings.css:59`, docs' `.settings-column`); mail at
`Settings.tsx:240-275`, docs at `:234-271`. **Calendar has the row but not the shell**: three rows in
the shared `Sheet` (`:33-107`). **Margin is a modal form**: `.overlay > .panel` with stacked `<Field>`
and uppercase small-caps labels (`:126-214`). They agree on the row and disagree on every name:
`.set-field`/`.set-field-label`/`.set-field-note` (mail, `:856-874`),
`.setting-row`/`.setting-label`/`.setting-note` (docs, `:67-88`),
`.setting-row`/`.setting-name`/`.setting-note` (calendar, `overlays.css:511-535`). Docs and calendar
are **one word apart**, and mail's is the only one naming the control slot and taking `children`
rather than baking a switch into the row.

- **Switch.** mail `ui/Toggle.tsx` (46 + 78 css), 9 uses; docs inline in `SettingRow` (`:74-85`), 3
  uses. Same `<button role="switch" aria-checked data-on>` with a knob on `translateX`; diffs are
  `data-on=""` vs `"true"`, 34x20 vs 38x22, two disabled treatments. margin uses a raw checkbox
  (`:202-205`); calendar has none.
- **Segment.** mail `ui/Segment.tsx` (47 + 76 css), 10 uses; calendar inline twice (`:46-63`, `:74-91`,
  ~39 lines). Same class names, **opposite visual models**: calendar paints the active option
  `--accent` (`overlays.css:567-570`), mail lifts it onto `--paper` in an `--accent-wash` track
  (`Segment.css:28-31`). `data-on` vs `data-active`; mail has `role="tablist"`, calendar no ARIA.
- **Select.** Nobody abstracted it, written five times: mail's `FontPicker` (`:876-923`), `TimePicker`
  (`:1099-1126`), `SwipePicker` (`:1573-1597`), margin's `FontSelect` (`:51-73`) and an inline language
  select (`:149-155`), all `<select className="settings-select">` with the same "unknown current value
  gets its own leading option" hatch; mail's font picker and margin's `FontSelect` are near-duplicates,
  both driven by `margin-shared/fonts`.
- **Text field.** mail has **two competing abstractions** and Settings uses neither consistently:
  `ui/Field.tsx` (84, unused there), a local commit-on-blur `Draft` (`:1000-1063`), `TextSetting`
  (`:1065-1097`), and three raw inputs.
- **Button.** mail `ui/Button.tsx` (65 + 96 css), variants `default|primary|ghost|danger`, 21 uses in
  Settings alone. Calendar expresses **the same vocabulary** as a CSS attribute,
  `.panel-button[data-variant]` (`overlays.css:60-135`); margin uses `.btn-primary`/`.btn-ghost`/
  `.btn-danger` (`app.css:1769-1800`); docs adds `.btn-quiet`. Four conventions, one control.

Open, close and escape are four mechanisms: local `useState` (margin, docs), an overlay store with a
back trail (calendar), a dedicated store (mail). Only mail has keyboard section nav, and it re-points
the app's own `j`/`k` at the rail to get it (`Settings.tsx:228-238`), which does not port.

**The honest split.** Generic chrome per file: mail ~230 of 2551 (9%), docs ~77 of 346 (22%), margin
~35 of 215 (16%), calendar 0 of 111 because its chrome is in `Sheet`. The other 1385 lines of mail's
file are Gmail scopes, IMAP servers, R2 buckets, mbox export and recovery phrases. Under 200 lines
saved across all four out of 3402, and a `SettingsShell` would have two consumers who disagree about a
header, a close button and a drag region. Ship the row primitives; leave the shell.

## Sidebar, resize, row menus, popup menus

Scope correction: **calendar has no sidebar and no resizable pane** (no `.pane-resizer`, no `--pane-*`
token, no `aside`) and **mail has no resizable pane either** (`--list-w` is a constant at
`styles/mail.css:40`, and `src/pane.ts` is visibility). Resize is a two-app problem.

**Resize.** margin `ResizeHandle.tsx` (52) + `panes.ts` (47) against docs `ResizeHandle.tsx` (78,
`panes.ts` inlined). The drag body is the same algorithm line for line: `setPointerCapture`, a flag on
the document, `pointermove` computing `startWidth + delta`,
`Math.round(Math.min(MAX, Math.max(MIN, px)))`, same MIN 200 / MAX 460 / DEFAULT 248. CSS
near-verbatim (`margin/app.css:140-186` against `docs/tree.css:64-108`).

Each has half the correct behaviour. margin has keyboard resize (`:28-37`, STEP 16, Home to reset), an
`aria-label` and `tabIndex={0}`; **docs' separator cannot be focused at all** (`:68-77`). Docs wraps
storage in `try/catch` (`:23-27`, `:31-36`, `:41-47`); `margin/panes.ts:41` throws on a webview that
denies localStorage. Neither handles `pointercancel` or calls `releasePointerCapture`, so a cancelled
pointer leaves the listeners attached and `cursor: col-resize` pinned on the document. Neither
debounces: a 120Hz drag issues 120 synchronous `localStorage.setItem` calls per second
(`margin/panes.ts:41`, `docs/ResizeHandle.tsx:24`), with no rAF anywhere. **Docs flashes 248px and
jumps**, its boot script restoring theme, sidebar and width but not `margindocs-pane-sidebar`, leaving
the width to a `useLayoutEffect` (`:40-48`). Sidebar open/closed is persisted in docs
(`Titlebar.tsx:38`) and not in margin (`EditorView.tsx:62`).

**Menus: six implementations of one object.**

| | positioning | edge | portal | dismiss | Esc | trap | restores focus |
|---|---|---|---|---|---|---|---|
| margin `RowMenu` (161) | anchor rect | **none** | yes | mousedown capture | yes | yes | yes |
| margin `Menu` (42) | CSS only | none | no | backdrop div | **no** | yes | via teardown |
| margin `AddPageMenu` (105) | anchor rect | none | no | mousedown | yes | yes | yes |
| docs `RowMenu` (187) | point | clamp both axes | yes | mousedown capture | yes | no | yes |
| docs `WidthMenu` (187) | CSS only | none | no | backdrop + blur | yes | no | keyboard only |
| mail `Popover` (108) | anchor rect | x clamp, manual `top-end` | no | pointerdown capture | yes | no | **no** |

`margin/RowMenu.tsx:33-37` sets `top: r.bottom + 4` with no clamping, so a row low in a long chapter
list opens a menu off the bottom of the window; `margin/Menu.tsx` has no positioning code and **no
escape layer**, so Escape does not close it; `docs/RowMenu.tsx:44-45` clamps both axes to an 8px inset
and thunks its `items` (`:29`) so a thousand-row tree does not build a thousand menus. The best
placement maths is calendar's, and it is not in a menu:
`calendar/src/components/EventDetailsModel.ts:71-104` is a pure, unit-tested function trying right,
left, below, above, then centre, clamping the cross axis and returning the side it chose. Two ideas
only one app has: docs' **caret bargain** (`WidthMenu.tsx:105`, `Titlebar.tsx:188`, `e.detail === 0`
detects keyboard activation and only then moves focus, mouse presses `preventDefault`ed so the caret
stays in the sentence), and mail's **reposition rather than close** on scroll and resize
(`Popover.tsx:63-69` plus a `ResizeObserver` on the body), which is right for a contact card in a
scrolling thread and wrong for a menu. Share the placement hook, not the dismissal policy.

**Sidebar chrome vs content.** margin `Sidebar.tsx` (310) is roughly 95-100 generic to 210 app; docs
`Sidebar.tsx` (523) + `FileTree.tsx` (282) roughly 150 to 370. The chrome markup is already textually
identical: `.sidebar` is the same nine declarations (`margin/app.css:188-199`, `docs/app.css:167-178`)
and `.nav-label` is byte-identical (`margin:235-241`, `docs:209-215`); mail's equivalent is
`ui/GroupHead.tsx` (25), calendar has none. Two more shared behaviours hide here: the **row drag
gesture** (`margin/Sidebar.tsx:99-131` and `docs/Sidebar.tsx:274-317`, the same 45 lines of slop
threshold, window pointermove/pointerup, `suppressClick`, `elementFromPoint`) and **roving focus**
(`margin:147-166`, `docs:217-255`). **Headers** are one object with different cargo
(`<header className="titlebar" data-tauri-drag-region>` at `calendar/Header.tsx:58`,
`mail/Header.tsx:76`, `docs/Titlebar.tsx:225`); mail and calendar both re-derive the "a button that
also drags swallows its own click" rule in comments, and mail alone carries the macOS double-click fix
(`Header.tsx:36-44`).

Shared, by payoff to risk: `definePane`/`<ResizeHandle>` (margin's parameterised shape, docs' storage
guards, plus the three fixes neither has); `useAnchoredPosition` on calendar's `place()`;
`<Menu>`/`<MenuAt>` on docs' `RowMenu` body; `<Popover>` kept separate; `useRowDrag`; `useRovingFocus`;
a thin `<SidebarShell>`. Stays: everything that knows what a row is, and all copy.

## The PDF export preview

The largest single-file duplicate in the tree: `margin/ExportPreview.tsx` (336) and
`docs/ExportPreview.tsx` (448). Docs says so at `:3-5`: "The sibling book app answers Export with this
same panel, and both apps answer it this way for the same reason." The same code, not the same idea:

- `interface Frame` and `measureEditorPane()`/`measurePane()`, querying `.editor-pane` and rounding its
  rect (`margin:21-38`, `docs:45-62`), then pinning the panel to it inline (`margin:150-157`).
- The toolbar: `.preview-bar` with a close icon button, `.preview-title`, `.preview-count`,
  `.preview-zoom` with the same `M5 12h14` and `M12 5v14M5 12h14` glyphs, `ZOOM_MIN 0.5`,
  `ZOOM_STEP 0.25`, and a `btn-primary` whose label is the same ternary,
  `saving ? "Saving…" : compact ? "Save" : "Save PDF…"` (`margin:177-179`, `docs:254-255`). Plus
  `.preview-warn`, `.preview-stage`, `.preview-loading` and the same "Typesetting…" copy.
- The fit arithmetic, character for character:
  `Math.max(240, Math.min(stage.width - 56, (stage.height - 56) / ratio))` (`margin:208`, `docs:344`).
- The lazy page renderer: a `ResizeObserver` on the stage, an `IntersectionObserver` per page at a
  1400px `rootMargin`, `Math.min(window.devicePixelRatio || 1, 2)`, a hand-built canvas with
  `className = "preview-canvas"`, `el.replaceChildren(canvas)`, a `task?.cancel()` teardown.
  `margin/PdfPage:263-336` against `docs/Page:365-448`. The only differences are names.

Roughly 200 of margin's 336 and 220 of docs' 448 are one component; app-specific are the compile call,
the save path and the warning text. `<PdfPreview bytes title onSave saving warning onClose />` plus the
`.preview-*` CSS: the cleanest large win, with no design argument attached.

## The shortcuts sheet

Three apps, two of them the same file. `calendar/src/keys/Shortcuts.tsx` (49) and
`mail/src/keys/Shortcuts.tsx` (53) open with the identical three-line comment ("The `?` sheet,
generated from the binding table. There is no list of shortcuts anywhere in this file, which is the
entire point"), both render `<Sheet size="wide">` around `GROUPS.map` over `BINDINGS` into
`.shortcuts-group > .shortcuts-heading + .shortcuts-list > .shortcuts-row > .shortcuts-keys +
.shortcuts-label`, and both close with a `.shortcuts-note` whose first sentence is word for word
"Nothing is modal and nothing is chorded." The CSS matches selector for selector
(`calendar/palette.css:120-176`, `mail/keys/shortcuts.css:3-51`). Mail's improvements: props instead of
`useOverlays` (`:11-14`), `<Key>` instead of a raw `<kbd className="key">`, and it drops keyless
bindings because "a sheet of shortcuts that lists one with no keycap beside it is a sheet that has lost
the plot" (`:23-24`). Docs' `components/Shortcuts.tsx` (69) is the earlier form: an inline
`.overlay`/`.panel` and a different vocabulary
(`.key-group`/`.key-list`/`.key-row`/`.key-what`/`.key-combos`/`.key-cap`). Margin has no sheet and no
binding table to generate one from. `<ShortcutsSheet open onClose bindings groups keyLabel note />`:
~170 tsx and ~130 CSS to one copy.

## Rich text

Two premises in the brief are wrong. **Calendar has no rich text editor.** `RichText.tsx` (84) is a
read-only renderer walking a pre-parsed node tree (`:32-73`); the only `contenteditable` in the tree is
a touch CSS selector (`app.css:49`), `useEditor.ts` (45) is a zustand store rather than tiptap's hook,
and descriptions are edited in a `<textarea>` (`EventEditor.tsx:493-499`). Calendar's real artifact is
`EventDetailsHtml.ts` (449), a hand-written sanitiser with a fixed tag vocabulary (`:22`, `:48+`),
written by hand rather than with `DOMParser` to stay pure and testable (`:13-15`): a display-and-defend
problem for HTML written by anyone who can put an event on a calendar you subscribe to, not a small
tiptap. Also not editors: `mail/FocusReply.tsx` (345) is a `<textarea>` by decision (`:29-31`),
`mail/MessageBody.tsx` (272) a sandboxed iframe (`:258-268`), `docs/FileViewer.tsx` (343) a pdfjs viewer.

So: three tiptap surfaces, and their extension lists are mutually incompatible for good reasons.
margin's `extensions.ts` (28) keeps StarterKit nearly whole and adds seven local extensions
(`Figure` 41, `ParagraphIndent` 42, `TextAlign` 64, `SearchHighlight` 233, `Proofing` 139, `Paste` 76,
`Shortcuts` 11). Mail's `Editor.tsx:41-49` is StarterKit with `heading:false` and
`horizontalRule:false` and **zero custom extensions**, not even Placeholder, using a sibling `<span>`
plus `data-empty` (`:85-86`, `editor.css:68-83`). Docs' `extensions.ts` (196) switches **seventeen**
StarterKit entries off (`:128-147`), keeping it only for undo/redo, drop cursor, gap cursor and list
backspace, then **generates** every node and mark at runtime from a frozen `src/model/schema.ts`
(`:60-87`, `:89-110`); its first twenty lines are a written argument against a shared extension list.

Content types are three (JSON, HTML string, markdown-backed PM node) with no `content` prop serving all
three. Lifecycle differs: margin holds one module-level singleton editor across every chapter
(`session.ts:23-42`) because a book is many documents; docs collapsed that cache into a path-keyed LRU
(`Editor.tsx:11-14`). Content sync is three mechanisms of three sizes: mail guards `setContent` with a
last-emitted ref, five lines (`Editor.tsx:52`, `:77-81`); margin never calls `setContent`, using
`view.updateState` off an LRU of `EditorState` (`session.ts:20`, `:44-59`); docs does the same plus a
fallback re-rendering a file as one raw block on schema failure rather than an empty doc, so a save
cannot destroy the file (`Editor.tsx:620-644`). Merging these produces something worse than any of
them. Nobody debounces inside the editor; all three do it in the shell at 800ms.

Toolbars: margin `FloatingToolbar.tsx` (198), docs `Toolbar.tsx` (938), mail **none by decision**
(`Editor.tsx:16-19`). Docs says at `:13-19` it is a port of margin's, and the `tool()` helper
(`margin:109-119`, `docs:156-173`) plus the `.tool-wrap` + backdrop + popover idiom are the same, but
docs' drives a hand-rolled `EditorHandle` (`editor/index.ts:89-136`) reading
`active.marks.includes("strong")` while margin's takes a `TiptapEditor` and calls `isActive`, kept
live by a `forceUpdate` on `"transaction"` (`:48-55`) docs deliberately did not port.

**No shared editor component, and not even a shared extension list.** The spirit has already been
shared by hand-porting with citations in the comments. Genuinely extractable: `SearchHighlight` plus
`searchStateOf` (`margin/editor/search.ts` 233 against `docs/editor/search.ts` 264, a 73-line diff,
docs' header at `:5-7` saying the only change of substance is a rename); `positions.ts` (64 vs 52);
the toolbar primitives; and the install-then-restore-position helper with its triple scroll apply
including `document.fonts.ready` (`margin/Editor.tsx:59-104`, `docs/Editor.tsx:606-690`). A small
editor kit, not an `<Editor>`.

## First run, onboarding, help

calendar `FirstRun.tsx` (38), `Accounts.tsx` (159); docs `Recents.tsx` (86), `DocumentSetup.tsx` (354);
mail `Onboarding.tsx` (208), `Connect.tsx` (333), `ConnectMail.tsx` (644), `Tour.tsx` (431),
`Guide.tsx` (113), `Help.tsx` (155), `ui/EmptyState.tsx` (16); margin `Library.tsx` (181) as its start
screen. **Margin has no first-run, welcome or tour screen at all.**

**There is no shared step-by-step setup flow.** Exactly one component in four apps has numbered steps
and it is a slideshow: `Tour.tsx:355` (`useState(0)`), `:392-404` (step dots), `:405-414` (the only
Back/Next pair anywhere), `:411` (the only terminal success screen). Everywhere else the flow ends by
unmounting and every other screen has a single primary button. `Connect.tsx` and `ConnectMail.tsx` look
like wizards and are not: their states are phases of an external process the user cannot navigate, and
`Connect` has no way back once the account is written (`:276-278`). Four screens appearing at the same
moment in a product's life, sharing an aesthetic, not a shape.

- **The empty-stage anatomy: four apps, four class vocabularies, one layout.** A mark, an `h1`, one
  line of prose, a row of buttons, one line of fine print. `mail/Connect.tsx:78-125`
  (`welcome-mark`/`welcome-title`/`welcome-line`/`welcome-actions`/`welcome-privacy`),
  `docs/Recents.tsx:40-57` (`start-title`/`start-line`/`start-open`), `calendar/FirstRun.tsx:19-33`
  (`first-run-title`/`first-run-note`), `margin/Library.tsx:109-123` (`card-action`).
- **The Google connect-pending block**, the strongest single duplication here.
  `calendar/Accounts.tsx:88-114` and `mail/Connect.tsx:136-171` are the same block: a "waiting in your
  browser" line, a note conditional on `authUrl`, and Open link / Copy link / Cancel wired to
  `openAuthUrl`/`copyAuthUrl`/`cancelConnect` on a store called `useAccounts` with the same phase
  names, both guarding Escape identically with
  `useEscapeLayer(phase === "connecting", cancelConnect)` (`Accounts.tsx:44`, `Connect.tsx:51`).
- **The recents shelf** (`docs/Recents.tsx:62-80`, `margin/Library.tsx:124-151`) and **the progress
  bar**, five copies of which four are inside mail (`Connect.tsx:310-318`, `Arriving.tsx:113`,
  `ListColumn.tsx:68-76`, `Settings.tsx:1135`, `docs/UpdateDialog.tsx:96`), two concepts sharing
  markup: sync progress and download progress.
- **Empty-list placeholders.** Mail extracted it (`ui/EmptyState.tsx`, 16 lines, one `<p>` and a 7-line
  rule). The others hand-roll a one-line paragraph under a different class each time: `.move-empty` and
  `.dock-empty` (margin), `.panel-empty` and `.palette-empty` (calendar), `.pane-empty` (docs).

Shared: `<Stage mark title line actions footnote>`, `<RecentList items renderRow onOpen onForget>`,
`<ProgressBar value label count>`, `<OAuthPending ready onOpen onCopy onCancel>`, `<EmptyState>`.
`<Slides>` only if a second app wants a tour. Stays: every phase machine, all copy, `ConnectMail`'s
644 lines of IMAP discovery, `DocumentSetup`'s font model, `Tour`'s nine slides.

## Virtualised lists and keyboard row navigation

Only mail virtualises (`react-virtuoso` at `ListColumn.tsx:2`, `:180`, `:451`); the other three render
everything and say so (`calendar/AgendaList.tsx:4-6`). Nothing to share about windowing.

A great deal to share about the keyboard, and this is the largest instance of copied code in the audit.
**Five distinct expressions for "move the selection by one", and the split is not by app:** clamp with
a seed from whichever end the delta came from, four near-identical copies
(`mail/store/useMail.ts:292-300`, `useFeed.ts:96-102`, `useScreener.ts:161-168`,
`calendar/store/useCalendarView.ts:96-97`); clamp with no seed, where `docs/Outline.tsx:100` and
`docs/Backlinks.tsx:92` are literally the same line and `Outline.tsx:95` says so, plus
`mail/Guide.tsx:53` and `mail/CommandPalette.tsx:164-165`; clamp by indexing off the end and guarding
`undefined` (`docs/Sidebar.tsx:225-240`, `margin/Sidebar.tsx:155-165`); true modulo wrap
(`docs/Palette.tsx:91-92`, `margin/Menu.tsx:28-30`, `margin/AddPageMenu.tsx:56`,
`margin/RowMenu.tsx:52`, `docs/RowMenu.tsx:80`, `docs/WidthMenu.tsx:116`, `docs/Titlebar.tsx:198`); and
wrap plus seed (`mail/Help.tsx:110-111` and `mail/MoreMenu.tsx:108`, identical lines).

margin-docs alone contains three of the five. A list wraps or does not depending on which app and which
surface you are in, which is precisely what a shared design language is supposed to settle.

Selection is four models: a key in a zustand store with DOM focus never moving and rows at
`tabIndex={-1}` (all mail lists, calendar's agenda); an index or id in local state
(`docs/Palette.tsx:57`, `mail/CommandPalette.tsx:321`, `mail/Guide.tsx:22`); roving DOM focus with a
`tabIndex` shadow (`docs/Outline.tsx:102`; `docs/Backlinks.tsx:94`; `docs/Sidebar.tsx:101-105` and
`margin/Sidebar.tsx:142-145`, the same rAF + `CSS.escape` + `querySelector` trick); and pure DOM focus
off `document.activeElement` with no index (every menu). Keeping the row on screen is five mechanisms,
and one found a bug the rest still carry: `docs/Outline.tsx:80-90` deliberately does **not** use
`scrollIntoView`, doing the arithmetic by hand, because as `:79` says it "is free to scroll every
ancestor of the row as well". The other four use it (`calendar/AgendaList.tsx:40-44`,
`mail/Feed.tsx:127-130`, `docs/Palette.tsx:75-77`, `mail/ListColumn.tsx:281-285`). Two gaps: **no
type-ahead anywhere in any app**, and Home/End exists in only three places, all trees or menus
(`docs/Sidebar.tsx:233-240`, `margin/Sidebar.tsx:156-159`, `margin/Menu.tsx:24-27`).

```
useRovingFocus(ref, { selector | refs, wrap, homeEnd, seedFromEnd, onMove })
stepIndex(count, at, delta, { wrap }): number | null
useScrollSelectedIntoView(scrollerRef, selectedId, { attr: "data-id" })
```

`useRovingFocus` has ten call sites, eight of them the same eight lines differing only in the CSS class
queried and whether they wrap: `margin/Menu.tsx:17-32`, `margin/AddPageMenu.tsx:51-59`,
`margin/RowMenu.tsx:48-54`, `docs/RowMenu.tsx:75-81`, `docs/WidthMenu.tsx:112-117`,
`docs/Titlebar.tsx:193-199`, `mail/Help.tsx:105-123`, `mail/MoreMenu.tsx:104-116`, plus
`docs/Outline.tsx:99-109` and `docs/Backlinks.tsx:91-101` on refs. `stepIndex` replaces seven copies;
the scroll hook replaces three and should use `Outline`'s manual arithmetic.

Stays: all ordering (`useCalendarView.ts:82-91`); ListColumn's virtualiser handle, so the shared hook
must take a scroll strategy rather than assume the DOM; the tree's ArrowLeft/ArrowRight expand-collapse
(`docs/Sidebar.tsx:241-251`); multi-select, only in mail (`ListColumn.tsx:209-222`). The hook must take
key predicates rather than hardcode `e.key`, because mail and docs route list keys through a remappable
binding table (`Guide.tsx:55-58`, `ListColumn.tsx:224-269`) while every menu listens for literal
`ArrowDown`. `margin/Library.tsx` is the one screen that would gain a feature rather than lose
duplication: a card grid with no arrow navigation, only Enter and Space (`:138-143`).

---

## Ranked, with the size of the win

| Rank | Cluster | Apps | Now | After | Argument needed |
|---|---|---|---|---|---|
| 1 | `Sheet` + `Confirm` + panel CSS | 4 | ~300 tsx, ~240 css | ~170, ~60 | none, mail already forked calendar |
| 2 | `useRovingFocus` + `stepIndex` + scroll hook | 4 | ~200 over 20 sites | ~60 | wrap or clamp must be settled |
| 3 | PDF export preview | 2 | 784 | ~450 | none |
| 4 | Primitives: Button, Toggle, Segment, Key, Field, Icon, EmptyState, Pill, GroupHead | 4 | ~660 tsx, ~600 css | ~410, ~350 | one class vocabulary wins |
| 5 | Find bar | 2 | 475 tsx, 370 css | ~320, 161 | none |
| 6 | Shortcuts sheet | 3 | ~170 tsx, ~130 css | ~60, ~50 | docs adopts the sheet |
| 7 | Palette shell + `commandMatches` + `highlight` | 3 | ~510 | ~250 | wrap vs clamp, row identity |
| 8 | Menu and anchored positioning | 4 | ~890 | ~450 | six behaviours to reconcile |
| 9 | Toast | 4 | ~140 | ~60 | two visual designs |
| 10 | `Stage`, `RecentList`, `ProgressBar`, `OAuthPending` | 4 | ~250 | ~130 | none |
| 11 | ResizeHandle and pane width | 2 | 177 | ~110 | none, plus three bugs fixed |
| 12 | `SearchHighlight` + `positions.ts` | 2 | ~590 | ~300 | none |
| 13 | Settings shell | 2 | ~340 | ~250 | not worth it, see above |

About 1600 lines of tsx and 900 of CSS collapse to roughly 900 and 500, but the number is not the
point. The four apps disagree about whether Escape closes a menu, whether a list wraps at its ends,
whether a separator can be focused, whether a repeated toast restarts its timer, and whether the safe
button gets focus in a delete confirmation, none of it visible until someone uses two of them in one
afternoon. And several clusters have one app that got a detail right and three that did not:
`docs/Outline.tsx:80-90` alone avoids `scrollIntoView` scrolling every ancestor, `docs/Palette.tsx`
alone has `aria-activedescendant`, `margin/ResizeHandle.tsx` alone has a focusable separator, and
`mail/store/useToast.ts` alone restarts on a repeat. Extraction is how those stop being luck.

## What should not be shared

- **Section bodies of Settings.** 1385 lines in mail alone, all Gmail scopes and mbox export.
- **Content matchers.** A Rust fzy scorer, SQLite FTS, substring over event fields, a backend parse.
- **Any `<Editor>` or shared tiptap extension list.** Three content types, three schema policies, three
  lifecycles, and `docs/editor/extensions.ts:1-20` argues against the list specifically. Likewise
  `calendar/RichText.tsx` (84) and `EventDetailsHtml.ts` (449), a read-only renderer and a sanitiser
  for HTML the app did not author.
- **Setup flows.** No shared wizard shape exists. Share the empty-stage anatomy and the OAuth pending
  block, not the flows. And **`src/width.ts`**, which shares a filename across margin and docs while
  meaning unrelated things; rename one instead.
- **Icon paths** beyond the handful in `margin-shared/icons`. Mail's `ui/icons.ts` (73) has 29 paths
  and shares five names with the shared set by re-export (`:14`), which is the right pattern.
- **`calendar/ColorPicker.tsx` (94) and `mail/ui/Avatar.tsx` (87).** Superficially both derive a
  colour; actually a Google `colorId` radio group and a hashed-hue initials badge. Similarly
  `margin/Dock.tsx` (223) is Typst compile plus device frames, only `DockHead` (`:60-70`) is chrome.
- **`ProofPopover`, docs 292 against margin 76.** Same feature and the same `.proof-pop` /
  `.proof-suggestion` / `.proof-action` classes, but docs has grown a keyboard walk, an escape layer, a
  focus-return policy and a flip-above fallback margin has not. Share the anchored-menu primitive
  underneath; leave the issue rendering in each app.
