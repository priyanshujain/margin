# The shared hooks: `@margin/hooks` and `@margin/ipc`

Scope: hooks, utilities, the theme, the keyboard registry, the escape stack, the IPC wrapper, the
updater and the zustand conventions. The React components that consume all of it are
[ui-kit.md](ui-kit.md); nothing here specifies a component. Short names as in
[design-system.md](design-system.md): `margin` = `python/margin`, `calendar` = `python/margin-caledar`,
`editor` = `rust/margin-editor`, `mail` = `rust/margin-mail`, under `/Users/pj/Workspace/projects`.
Line numbers and hashes are as of 2026-09-06; the audit behind this is
[.research/frontend-utils.md](.research/frontend-utils.md).

## One missing dependency line, and a template that already works

Calendar is not wired to the shared package at all, in TypeScript or in CSS. There is no
`margin-shared` in `python/margin-caledar/package.json`, where margin has it at line 26
(`file:./shared`), editor at 33 and mail at 27 (`file:../../python/margin/shared`), and
`python/margin-caledar/src/styles/tokens.css` does not import the shared tokens the way
`python/margin/src/styles/tokens.css:3`, `rust/margin-editor/src/styles/tokens.css:5` and
`rust/margin-mail/src/styles/tokens.css:6` do. Every extraction below is blocked on that one line,
because a hook calendar cannot import is a hook that gets copied instead. Nothing else is needed for
build config: no app uses tsconfig path aliases, so a dependency plus Vite's own resolution is the
whole mechanism.

The migration shape does not need inventing either. **`python/margin/src/model/fonts.ts`** (41 lines)
and its twin `rust/margin-editor/src/model/fonts.ts` (40) are already re-export shims: they pull the
catalogue from `margin-shared/fonts`, declare one app-local alias so call sites keep the app's own
noun (`BookFonts` in margin, `DocumentFonts` in editor), and carry a header saying why the catalogue
is not theirs. Two apps, one module, in production today. Every module below follows it: the app's
file stays where it is and becomes a re-export, no call site moves, and the shim is deleted later only
if it holds no alias worth keeping.

## Byte-identical today, verified by hash

Compared with `md5`, not by reading them. Nothing in this section is a judgement call.

| Module | Apps | Lines | Hash | Note |
| --- | --- | --- | --- | --- |
| `src/escape.ts` | margin, calendar, editor | 36 each | `3b1f67d691647be7d61a23a5acd96a7b` | whole file, identical |
| `src/escape.ts` | mail | 40 | `6d9abcfb42b082961110af9a33647f19` | four-line header only |
| `src/escape.ts` | all four | 148 | `9c8530a09b35e5d697bb2b95674a2e73` | comments stripped |
| `useMediaQuery` | all four | 13 each | `c3499ac5ee3d1a12b138d71b2cc787ed` | the function, not the file |
| `usePhone` | calendar, editor, mail | 17 each | `d565f08a457563fd8ef874a6435a2bbd` | |
| `useTouch` | calendar, editor, mail | 17 each | `f4285e4c...` | |
| `src/theme.ts` | margin, calendar, mail | 16/16/21 | `ab9c99ff29565c686028e16960274d9d` | comments stripped, key normalised |
| `src/store/useTheme.ts` | margin, calendar | 16 each | `b48a3bccfadd21b9bb4efa91eef5041c` | mail's 17 add a `set(theme)` |
| `src/store/useToast.ts` | calendar, editor | 15 each | `dfc90357bbcfbbfe71ffb8ba0e680c00` | |
| `src/ipc.ts` lines 1 to 41 | calendar, editor | 41 each | `afde11927f75a3a81b2b459658c5b7be` | doc comments included |
| `call<T>` body | calendar, editor | 8 each | `b4259488afdd6a0f154b8e86eae1ff9c` | |
| the platform block | calendar `keys/bindings.ts:76-87`, editor `:206-217`, mail `:521-532` | 12 each | identical | `isMac`, `PRIMARY_LABEL`, `primaryHeld`, `secondaryHeld` |

`src/store/useOverlays.ts` is the near miss worth naming: calendar's 64 lines and mail's 72 differ only
in the `Overlay` union and one reflow of `push`, and all seven actions are character-identical. Mail's
header already says "Ported from the calendar's store of the same name."

**The plan is ordered by confidence and mechanical safety, not by line count.** The theme is the
largest single win and it is ninth, because it changes behaviour in three apps. The keyboard registry
is the highest-value item here and it is twelfth, because it needs two type parameters that do not
exist yet. What goes first is what can move without anybody having to think.

| # | Item | Size | Risk |
| --- | --- | --- | --- |
| 1 | `escape.ts` | ~148 | none, identical in four |
| 2 | `createOverlays<T>()` | ~130 | none, identical in two |
| 3 | `platform.ts`, both halves | ~50 | none in three; fixes two margin bugs |
| 4 | `onAppEvent` | ~60 | low; unlocks browser tests for two apps |
| 5 | media and layout hooks | ~190 | low |
| 6 | `useToast` and `notify` | ~60 | low; strict superset, fixes a timer bug in two |
| 7 | `makeStorage`, `rootSetting` | ~120 | low; deletes nine copies in editor, fixes six unguarded reads |
| 8 | `call<T>` and the `ipc.ts` preamble | ~50 | low |
| 9 | the theme, from editor | ~250 | medium; needs a per-app table and a generated boot script |
| 10 | small utilities | ~250 | medium; several user-visible inconsistencies get picked |
| 11 | `useFocusTrap` | ~56 | medium; a fix, not a saving |
| 12 | the keyboard registry, from mail | ~900 | high; needs `KeyContext` parameterised |
| 13 | the update store and driver | ~490 | high; four designs to reconcile |

Items 1 to 8 are mechanically identical across apps today and land with no behaviour change.

## The modules

The winners are spread across all four apps, and that is the point: the temptation is to name one app
the reference implementation and take everything from it, which would ship three regressions. Editor
wins the theme and storage, mail the registry, the toast and the IPC wrapper, margin the focus trap and
relative time, calendar the media hooks, the date maths and the updater's package-manager guard.

### The escape stack, from any of the four

A module-level stack of Escape handlers behind one lazily bound capture-phase listener, so Escape
unwinds layers in the order they went up instead of every component thinking it owns the key. Moves
verbatim, keeping mail's four-line header, the only copy that explains itself.

```ts
export function useEscapeLayer(active: boolean, onEscape: () => void): void;
```

The app supplies nothing. All three keyboard registries already defer to it by name rather than
handling Escape themselves, so extracting it does not touch them. Its
`const latest = useRef(onEscape); latest.current = onEscape;` is an inline `useLatest` that falls out
for free; export it when a second caller appears, not before.

### Media and layout, from calendar

Calendar's `src/useMedia.ts` wins over editor's identical file only because calendar has to adopt the
package anyway. Mail has no `useCompact`, and margin's is the bare one-liner
`useMediaQuery("(max-width: 899px)")` at `python/margin/src/useMedia.ts:15` without the `data-compact`
write, so the responsive layout described in [guidelines/platform.md](guidelines/platform.md) is
currently true of calendar and editor only.

```ts
export function useMediaQuery(query: string): boolean;
export function useCompact(): boolean;   // also writes data-compact on the root
export function usePhone(): boolean;
export function useTouch(): boolean;
export const PHONE_QUERY = "(max-width: 640px)";
export const TOUCH_QUERY = "(pointer: coarse)";
```

The app supplies its `index.html` boot script's copy of the same two queries, which calendar's comment
at `useMedia.ts:27` already flags as needing to stay in step. Same problem as the theme, same answer:
generate the boot script from the constants.

### The theme, from editor

`rust/margin-editor/src/theme.ts` is 167 lines against margin's and calendar's 16 and mail's 21, and
every extra line earns it. A seven-palette table with a `scheme` per row (`:35`), so light and dark are
properties of a theme rather than the only two themes. A real tri-state `ThemeChoice = Theme | "system"`
(`:20`) with a remembered light/dark pair under `margindocs-theme-light` and `margindocs-theme-dark`
(`:60`, `:61`), so choosing System does not forget which dark theme you liked. `storedPreference()`
(`:137`) dropping a stored id that no longer names a theme: without it a palette retired between
releases leaves the root carrying a `data-theme` no stylesheet answers, which is not one broken colour,
it is all of them. `watchSystemScheme()` (`:164`), the only live `prefers-color-scheme` subscription in
any of the four. `saved()` and `store()` (`:105`, `:115`) guarding `localStorage` behind `typeof` and
`try`/`catch`. And `theme.test.ts`, 172 lines, the only theme test in the family, holding the
TypeScript table and the boot script's copy of it to one table.

Mail fakes the tri-state at the UI layer: "System" deletes the stored key (`forgetThemeChoice()`,
`rust/margin-mail/src/appearance.ts:41`). That is a snapshot, not a subscription. Pick System at night
and mail stays dark through the morning.

```ts
export type Scheme = "light" | "dark";
export interface ThemeInfo<T extends string> { id: T; label: string; scheme: Scheme }
export interface ThemeConfig<T extends string> {
  themes: readonly ThemeInfo<T>[];
  keyPrefix: string;                       // "margindocs", "marginmail", ...
  fallback: Record<Scheme, T>;
}
export function createThemeStore<T extends string>(config: ThemeConfig<T>): UseBoundStore<StoreApi<ThemeState<T>>>;
export function bootScript<T extends string>(config: ThemeConfig<T>): string;
```

The app supplies the table, the key prefix and the fallback pair. Generating the boot script from the
same config is the part to insist on: all four apps hand-maintain an inline `<script>` in `index.html`
re-reading the theme keys before the bundle exists, and editor re-encodes its whole palette-to-scheme
table there in plain JavaScript.

### Storage and root settings, from editor

Bigger than the theme, and what makes the theme extraction tractable. Read a boot attribute first
because `index.html` already applied it, fall back to `localStorage`, apply to the root, write back.
Six copies: `src/theme.ts` in all four on `data-theme`; `rust/margin-mail/src/pane.ts` (20 lines) on
`data-no-pane`, whose header says it is "exactly the shape `src/theme.ts` uses";
`rust/margin-editor/src/width.ts` (52) on `data-width`; `python/margin/src/width.ts` (28) doing the
same job through a `--measure` custom property; `python/margin/src/panes.ts` (47) on `--pane-sidebar`
and `--pane-dock` with its own private `clamp`; `rust/margin-mail/src/appearance.ts` (43) on
`--font-ui`, `--font-heading` and `--body-size`.

```ts
export function makeStorage(prefix: string): {
  readString(key: string, fallback: string | null): string | null;
  readJson<T>(key: string, fallback: T): T;
  write(key: string, value: string): void;
  remove(key: string): void;
};
export function rootSetting<T extends string>(opts: {
  attribute: string; key: string; values: readonly T[]; fallback: T;
}): { initial(): T; apply(value: T): void };
```

The prefix is the only per-app parameter: every key in every app is already `<slug>-<setting>`, slugs
`margin-`, `margincal-`, `margindocs-`, `marginmail-`, 8 keys in margin, 6 in calendar, 17 in editor, 9
in mail. Editor wins because it has written the guarded accessor pair nine times inside one app
(`theme.ts:105`, `store/useUpdate.ts:64,73,85`, `store/useProofing.ts:102,111`,
`store/useDocumentFonts.ts:58,77`, `workspace.ts:60,70`, `width.ts:33`) with the same catch comment
four times, the verb swapped each time. The rule all six obey is stated in three files and should be
written once: a store may not touch the DOM, and a layout fact the stylesheet needs on first paint has
to be an attribute on the root, not a class on a component.

Do not reach for zustand's `persist` middleware or a `useLocalStorage` hook. The boot script reads
these keys before any bundle exists, so the stored shapes stay plain and hand-chosen;
`rust/margin-mail/src/pane.ts:1-8` and `appearance.ts:1-7` both document the constraint. `width.ts`
itself is not shared: margin has four named widths on a CSS variable, editor five on an attribute plus
command wiring, and only the persistence overlaps.

### The keyboard registry, from mail

Three reasons, all of them things calendar and editor cannot express.

`resolve()` layers contexts instead of replacing them (`rust/margin-mail/src/keys/keymap.ts:94-109`).
A `screener` or `focus` frame overrides only the keys it declares and leaves the base `view` keymap
live underneath; only `overlay` and `editor` shadow wholesale, through an explicit `SHADOWS_VIEW` list
at `keymap.ts:94`. Calendar and editor fall from the top frame straight to `global`, which kills the
base keymap the moment any non-overlay frame is pushed. Mail's comment at `keymap.ts:90-92` records
that it was written that way once, and that this is the bug the comment exists to prevent.

`normalizeCombo` (`keys/bindings.ts:545`) carries shift in the prefix alongside `cmd`, `ctrl` and
`alt`, and handles the two combos a naive split breaks, a key of `+` and a key of space. Calendar's and
editor's filter shift out entirely.

`bindings.ts` does not import `commands.ts`, so the table, the sheet and the palette are testable
without booting a store or Tauri; in calendar and editor the dependency runs bindings to commands to
every store, which is why editor's `commands.ts` is 433 lines and calendar's 224. Mail's is 75 with
zero store imports: a `Map<CommandId, Handler[]>`, `registerCommands` returning its own teardown, last
registered wins, so a screen takes over a verb on mount and hands it back on unmount. Editor's
`onCommand` fan-out is worth keeping alongside the stack for cases needing several listeners, and
`menu.ts` comes from editor, the same three lines everywhere but the only one exporting its id list and
testing that every menu id names a real command.

```ts
export function createKeymap<C extends string, K extends string>(opts: {
  bindings: readonly Binding<C, K>[];
  shadowsView: readonly K[];
  run: (command: C) => boolean;
}): { attach(): () => void; pushContext(context: K): () => void; useKeyContext(context: K, active?: boolean): void };
export function createCommands<C extends string>(): {
  register(handlers: Partial<Record<C, Handler>>): () => void;
  run(command: C): boolean;
};
export const PRIMARY_LABEL: string;
export const primaryHeld: (e: { metaKey: boolean; ctrlKey: boolean }) => boolean;
export const secondaryHeld: (e: { metaKey: boolean; ctrlKey: boolean }) => boolean;
export function normalizeCombo(combo: string): string;
export function comboLabel(combo: string): string;
```

The app supplies `BINDINGS` (20 rows in calendar, 20 in editor, about 70 in mail), the `CommandId`
union, `GROUPS`, `MENU_IDS`, the command implementations and its own `KeyContext` members. `KeyContext`
becoming a type parameter and `SHADOWS_VIEW` an app-supplied list is the whole cost of the extraction.

No app supports chords. All three registries carry the same header line, "Nothing is chorded and
nothing is modal. Two keys never combine into a third meaning." No pending prefix, no timeout, no
sequence buffer, so a `g i` binding is new work, and mail's `Map<string, Binding[]>` index with its
single `comboOf` extends to it most cleanly.

Margin has no registry and four unrelated mechanisms instead: an `if`/`else if` chain on a capture
phase window listener at `python/margin/src/components/EditorView.tsx:167-192`, a second competing
window listener for Cmd+K at `src/editor/FloatingToolbar.tsx:57-70`, TipTap extension shortcuts in four
files, and two hand-rolled `menu-action` chains at `App.tsx:25-35` and `EditorView.tsx:199-206` unaware
of each other. It also has no platform detection at all, so its five `e.metaKey` tests are macOS-only
and silently dead on Linux and Windows; `primaryHeld` fixes that for free. The blocker on the rest is
that margin has no palette and no shortcut sheet, so the generated sheet, which is most of the payoff,
has nowhere to land. Adopt `platform.ts` now and the registry when the palette exists.

The two `isMac` regexes differ by design and confusingly little: `keys/bindings.ts` uses
`/mac|iphone|ipad/i` because a Mac keyboard layout is what it asks about, `ipc.ts` uses `/mac/i` gated
on `isDesktop` because a title bar inset is what it asks about. Both are correct. One platform module
exports both, with an injectable user agent so the non-Mac branch is finally testable; no test
exercises it today in any app.

### The toast, from mail

Mail's 34 lines are a strict superset of the byte-identical 15 in calendar and editor: an optional
`ToastAction { label, keycap?, run }` for undo, and a `seq` counter bumped on every notice.

```ts
export interface ToastAction { label: string; keycap?: string; run: () => void }
export const useToast: UseBoundStore<StoreApi<ToastState>>;
export function notify(message: string, action?: ToastAction): void;
export const DISMISS_MS: number;
```

The counter is a real fix. Auto-dismiss lives in the component in all three, and calendar's effect deps
are `[message, dismiss]` (`python/margin-caledar/src/components/Toast.tsx:14`) where mail's are
`[message, seq, dismiss]` (`rust/margin-mail/src/screens/Toasts.tsx:23`), so in calendar and editor
notifying the same string twice does not restart the countdown and the second toast inherits the
remainder of the first one's timer. The dwell times differ for no reason: 4200ms
(`rust/margin-editor/src/components/Toast.tsx:4`), 5000ms (calendar `:4`), 6000ms (mail
`Toasts.tsx:6`). Pick one in the package and let an app override it only with a reason in a comment.
Margin has no toast, only a private `notify` at `src/store/useBackup.ts:45` writing into `useBook`'s
notice field; it adopts the store when it takes the primitive from [ui-kit.md](ui-kit.md).

### The focus trap, from margin

`python/margin/src/focus.ts`, 56 lines, is the only implementation in the family and the winner by
default: a `FOCUSABLE` selector filtered for hidden and detached nodes, Tab wrapping both directions,
opener restore on teardown, and a module-level counter behind `focusTrapped()` so the global key
handler can stand down.

```ts
export function useFocusTrap(ref: RefObject<HTMLElement | null>, active = true): void;
export function focusTrapped(): boolean;
```

Twelve call sites across ten files in margin today, plus two `focusTrapped()` reads at
`components/EditorView.tsx:125,168`. The app supplies a ref and, for popovers, the open flag. What it
fixes elsewhere is its own section below.

### The updater, from editor, with calendar's guard

Four apps, four answers. Editor's `src/update.ts` (171 lines) plus `src/store/useUpdate.ts` (129) wins:
named phase transitions (`begin`, `offer`, `progress`, `installing`, `failed`, `dismiss`) rather than a
generic `set(partial)`; `total: number | null` so a missing content length draws an indeterminate bar
instead of nought percent forever; version and notes as plain strings so no Rust resource handle sits
in the store, which is the mistake `python/margin/src/store/useUpdater.ts` makes; a launch delay and a
24 hour interval for automatic checks; a flush of the pending save before `relaunch()`; and a
discriminator for "this dev build has no updater plugin" so that reads as a sentence about the build
rather than an error. Fold in calendar's `packagedBy()` guard and `updateHint()` from
`python/margin-caledar/src/keys/updates.ts:13-33`: calendar is the only one of the four that tells a
nix or homebrew install to update through its package manager.

```ts
export function createUpdater(opts: {
  appName: string;
  packagedBy?: () => Promise<string | null>;
  launchDelayMs?: number;
  intervalMs?: number;
  beforeRelaunch?: () => Promise<void>;
}): { store: UseBoundStore<StoreApi<UpdateState>>; check(manual: boolean): Promise<void>; install(): Promise<void> };
```

The app supplies its name for the copy, its `packagedBy` command if it has one, and its
`beforeRelaunch` flush. Mail has no update module at all: `checkForUpdates` is inline at
`rust/margin-mail/src/App.tsx:98-118`, with a second differently shaped copy at
`src/screens/Settings.tsx:2478-2499`.

### `onAppEvent`, from mail

Nobody wraps Tauri's `listen`, and it shows. Mail is the exception, at
`rust/margin-mail/src/App.tsx:69-77`.

```ts
export function onAppEvent<T>(name: string, handler: (payload: T) => void): () => void;
```

It returns a synchronous unsubscribe, so each effect is one line, and it falls back to
`window.addEventListener` for a `CustomEvent` of the same name outside Tauri, which is what lets
Playwright drive mail's connect flow in a browser. Calendar hand-rolls a four-`.then(stop => stop())`
teardown at `python/margin-caledar/src/App.tsx:52-86`; editor uses a third pattern, a module exporting
`startWorkspaceEvents(): () => void` that the shell mounts
(`rust/margin-editor/src/workspace.ts:366-378`). The names are already common property: `menu-action`
in all four, `auth`, `sync-progress` and `store-changed` in calendar and mail, `pdf-warnings` in margin
and editor. Lift mail's nine lines, keep editor's "a module exports `start*()`" convention on top.

### The small utilities

Individually trivial, collectively about 250 lines and several user-visible inconsistencies. Each app
keeps its own call sites; only the implementation moves.

| Helper | Winner | Why that one |
| --- | --- | --- |
| `clamp`, `clampIndex` | `calendar/components/EventDetailsModel.ts:45` | six definitions and twelve inline sites today; only this one guards an inverted range, and half the callers pass `length - 1`, which goes negative on an empty list |
| `fileSize` | `mail/screens/format.ts:52` | four implementations, three spellings; 1536 bytes renders as "1.5 KB", "2 kB" and "2 KB" in one product family |
| `relativeTime` | `margin/src/time.ts:1` | the only one guarding clock skew with `Math.max(0, now - ts)`, and flooring, which is right for "how long ago"; editor's rounding makes 89 seconds "just now" and 91 seconds "2 minutes ago" |
| `useTick`, `useMinuteTick`, `useHourStart` | `calendar/src/useClock.ts` | re-schedules against the wall clock and reads `visibilitychange` and `focus` as ticks, because a timer's deadline is measured in time the machine spent awake |
| `rowTime`, `messageTime` | `mail/screens/format.ts:34-49` | kept beside `relativeTime`, not merged into it: the only one bucketing by local midnight, which is the difference between "Yesterday" being right and being wrong every evening |
| `plural` | `margin/store/useBackup.ts:56` | same function under a different name at `editor/linkRewrite.ts:694`, plus ten open-coded sites |
| `positions.ts` | `editor/src/editor/positions.ts` | scroll and selection restore; adds the `LIMIT = 200` LRU trim margin's unbounded map lacks, on a flat string key margin composes as `${bookId}/${chapterId}` |
| `useDebounced<T>(value, ms)` | new, from six identical effects | the class-field and module-level timers stay put, their cancel and flush semantics are the point |
| `dateFormat(options)` | `mail`, cached module constants | the same `{ day: "numeric", month: "short" }` formatter is constructed in six mail files |
| `latestOnly`, `deferred` | new; editor's counter, calendar's promise bridge | covers the stale-response race the two search stores solve differently |
| `parseJson<T>(text): T \| null` | new | for text off disk; `readJson` on the storage helper covers the eight guarded `localStorage` parses |
| `useResize` | needs `mail/screens/MessageBody.tsx:207`'s null guard | thirteen raw `ResizeObserver` instantiations, no hook, and only that one comment says why the guard is there |
| `scrollIntoViewIfNeeded` | `editor/components/Outline.tsx:79-89` | the only one that does not scroll every ancestor, already duplicated once inside editor |
| date maths | `calendar/src/time.ts` | `startOfDay`, `addDays`, `isSameDay`, `toDateOnly`, `parseDateOnly`; `startOfDay` alone is written three times across calendar and mail |

Calendar's `time.ts` avoids `Intl` entirely with hand-written day and month arrays. That is a deliberate
product decision for the grid and it stays in calendar; only the date maths moves.

### The zustand conventions

Nothing to extract, and worth writing into the package README because it is currently retyped in every
store. All 42 stores across the four apps use plain `create<State>((set, get) => ({ ... }))`; not one
uses the curried `create<T>()(...)`. No middleware anywhere: zero hits for `persist`,
`subscribeWithSelector`, `immer`, `devtools`, `useShallow` or `zustand/shallow`, and nothing is
imported from `zustand` except `create`. Selectors are uniformly inline, `(s) => s.field`, one hook
call per field rather than one destructured object, which is why no shallow comparator is needed:
every subscription is to a primitive or a stable reference.

Two conventions are universal and belong in the docs rather than in code: `if (!live()) return;` as the
first line of every backend-touching action (31 uses in mail, 6 in calendar, 1 in editor, 0 in margin,
which has no `live()` to call), and `error: String(e)` in state, never `e.message` (11, 6, 56, 21). The
async action shape repeats about 97 times (margin 1, calendar 10, editor 33, mail 53): optimistic set,
try, replace with the server's answer, catch, roll back and a "Could not ..." toast. Do not abstract
it. The bodies are five lines and every message is bespoke. Share the `Phase` union and `describe(e)`,
nothing more.

Three stores look shareable by name and are not. The three `useSearch` stores do three different jobs.
`useAccounts` in calendar and mail shares one real idea, an OAuth consent promise bridged over a Tauri
event with the resolver stashed in state, but it is on its third hand-copy
(`margin/store/useBackup.ts:70` to calendar to mail) and is about 60% app-specific: share the bridge
(`deferred`) and `openAuthUrl`/`copyAuthUrl`, character-identical in both and pure utility. Mail's
`useSync` is the best sync store because roughly 160 of its 210 lines are pure exported functions over
`SyncStatus[]` with a 9.6KB test behind them, but the store around them is mail's.

## `@margin/ipc`

Three of the four apps already agree on the target and it is worth stating as the target: a frozen
`src/ipc.ts` holding the DTOs that mirror `src-tauri/src/dto.rs`, plus `call<T>`, with thin per-domain
modules in `src/api/` that do nothing but name a command. Calendar has 6 api modules over a 41-line
preamble, editor 9, mail 16; mail's `ipc.ts` is 760 lines because it has the most DTOs, not because it
is structured differently. Only mail states the rule out loud, at `ipc.ts:4`: "Nothing outside src/api
may call `call` directly." That sentence goes in the package README. The DTOs stay per-app permanently;
they mirror one Rust file per app and there is nothing shared about them.

```ts
export const isTauri: boolean;                       // "__TAURI_INTERNALS__" in window
export const isDesktop: boolean;                     // isTauri and not a mobile OS
export const isMacDesktop: boolean;                  // isDesktop and a Mac user agent
export const live: () => boolean;                    // isTauri or import.meta.env.DEV
export type Phase = "idle" | "fetching" | "error";
export function describe(e: unknown): string;
export function createCall(opts: { mock?: () => Promise<MockIpc> }): <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
```

The three booleans are not interchangeable, and the doc comments calendar wrote at `src/ipc.ts:7-33`
say why: `core:window:*` sits in the desktop-only capability, so on a phone those commands are refused
rather than absent, and a Tauri event fires on mobile too.

**The naming disagreement.** `python/margin/src/ipc.ts:3` declares `export const isDesktop` and
computes exactly what the other three call `isTauri`. The same identifier means the opposite thing in
two of the four apps, and in margin it gates `runWritingTool`, `listSystemFonts` and
`gdriveListBackups`. [guidelines/platform.md](guidelines/platform.md) already records the confusion in
its own words. `isTauri` wins because it says what it tests. Margin renames on adoption, and those
three calls take `isTauri`, not the mobile-safe `isDesktop` they never meant.

**The phase union.** [guidelines/errors-and-feedback.md](guidelines/errors-and-feedback.md) requires a
string phase union on every handler that waits, with `data-phase` or `data-busy` on the control and a
present-tense label. `Phase` is the base union and an app widens it with domain members rather than
inventing a parallel one; editor's six-member `UpdatePhase` at `store/useUpdate.ts:26-33` is the worked
example, and its comment about why there is no "done" member is the reasoning to copy. The primitives
in [ui-kit.md](ui-kit.md) consume `Phase` directly, which is what stops an app forgetting the busy
state.

**The logging rule.** Every IPC failure has to reach the app's log file on disk, and a single transient
failure must never toast. `call` is where that is enforced, and mail is the only app enforcing it, at
`rust/margin-mail/src/ipc.ts:752-758`: the rejection is written to Rust with `log_note` before being
rethrown, and the write is skipped when the command is `log_note` itself, which is the recursion guard
nobody would re-derive. Those eight lines are why mail's `call` wins outright over the byte-identical
calendar and editor versions. `call` logs and rethrows; it never swallows and it never notifies. The
caller decides whether a failure is worth a sentence.

**The error shape.** Today every rejection crosses the boundary as a string and the convention is
`error: String(e)` in state, which is why `describe(e)` is the whole shared surface for now. When the
`margin-ipc` crate in [rust-crates.md](rust-crates.md) lands its serde error type, the TypeScript
mirror and an `isCommandError` guard belong here and `describe` narrows through it. Do not invent the
union before the Rust side has one.

**The dev fixture stub.** The mock branch inside `call<T>` is shared; the fixtures behind it are not.
Calendar, editor and mail each have `src/dev/fixture.ts` and `src/dev/mockIpc.ts` at 222/134, 344/696
and 1572/1769 lines, every `mockCall` a switch on `command` ending in the same byte-identical
`dev mock has no handler for ${command}` throw, with entirely app-specific bodies. `createCall` takes
the app's loader as `opts.mock` and keeps the `import.meta.env.DEV && !isTauri` gate that compiles the
branch out of a production bundle. The harness itself, `createMockIpc(handlers)` and the fixture
loader, is [testing.md](testing.md)'s and `@margin/test`'s; it is named here only so the seam is
unambiguous. Margin has no dev harness and cannot be driven in a browser at all, which is what adopting
`createCall` unlocks for it.

While in there: `python/margin-caledar/src/ipc/` exists and is empty. Delete it.

## The five bugs found on the way

| Bug | Where | What breaks, and for whom | When |
| --- | --- | --- | --- |
| One key, two constants | `mail/src/theme.ts:8` and `mail/src/appearance.ts:14` both declare `marginmail-theme` | a rename in one file silently stops the other reading the value, and appearance is the writer. Mail only | before: the theme extraction touches both files |
| Self-update over a package manager | `mail/src/App.tsx:98-118` and `mail/screens/Settings.tsx:2478-2499`; mail calls `packagedBy()` at `Settings.tsx:2475` and uses the answer only to print it at `:2516` | a nix or homebrew install downloads and writes over a path it does not own. Mail, on any install that is not the DMG | before: it is a write into another program's files |
| Two chords, one binding | `calendar/keys/bindings.ts:90-96`: line 94 filters shift out of the modifier prefix, line 95 lowercases the key | `Cmd+Shift+F` and `Cmd+F` normalise to the same string, so one silently wins and the other is dead. Calendar; editor's `:224-230` preserves case instead, which works for letters on a US layout and breaks for punctuation that only exists shifted | rides along: mail's `normalizeCombo` is the fix |
| The IPC gate misnamed | `margin/src/ipc.ts:3` | `isDesktop` computes "is Tauri" and gates three IPC calls. Margin is desktop-only today so it does not bite yet, and it bites the day it ships to a phone | rides along: the rename is the adoption |
| Unguarded disk parses | `margin/src/library.ts:31` and `margin/src/project.ts:19` | `JSON.parse` on a file read off disk, cast straight to `Book`. `library.ts` has a `try` and rethrows a readable sentence; `project.ts:19` has neither, so opening a truncated or hand-edited project file throws unhandled out of `openBook`. Margin | before: it is four lines |

The two disk parses are the tail of a wider pattern the storage helper closes: unguarded
`localStorage` reads at `margin/theme.ts:8`, `calendar/time.ts:16`,
`calendar/store/useCalendarView.ts:13`, `mail/theme.ts:13`, `mail/pane.ts:14` and
`editor/components/Outline.tsx:30`. Several run during module initialisation, so in a webview with
storage denied they take the whole app down on a throw, and calendar's is already why
`components/overlayModel.test.ts:20-31` has to stub a global.

## The accessibility gap

The one item here that is a fix rather than a saving.

Margin is the only app with a focus trap, and margin ships no `role="dialog"` and no `aria-modal` at
all. The three apps that do declare modal dialogs have nothing behind the declaration: calendar in
`components/overlayShell.tsx` and `palette/CommandPalette.tsx`, editor in eight files
(`ConflictDialog`, `Settings`, `Palette`, `ConfirmDialog`, `ExportPreview`, `DocumentSetup`,
`Shortcuts`, `UpdateDialog`), mail in `ui/Palette.tsx` and `ui/Sheet.tsx`. Calendar has autofocus and
no restore (`overlayShell.tsx:51-56`); editor and mail have an imperative `.focus()` on mount and no
trap. `aria-modal="true"` is a promise to assistive technology that the rest of the page is inert, and
in those twelve components it is not true: Tab walks straight out of the dialog into the list behind
it, and closing the dialog drops focus on `body`.

`useFocusTrap` moving into `@margin/hooks` is what makes the promise true, and it lands in all three
apps at once through the `Dialog` and `Sheet` primitives in [ui-kit.md](ui-kit.md) rather than through
twelve separate edits.

One wrinkle to settle first. Margin's module-level `focusTrapped()` and the other apps' keyboard
context stack are two mechanisms for one idea, "something in front owns the keyboard". Either the trap
pushes an `overlay` frame onto the keymap stack when it engages, which is the smaller change and makes
margin's two `focusTrapped()` reads disappear, or the shared keymap takes an injectable "external
owner" predicate. Prefer the first: one mechanism, and it is the one three apps already have.

## What the audit looked for and did not find

Confirmed absent from all four: any debounce or throttle utility; deep equality; a `safeParse` or
`tryParse` wrapper; class-name joining; `measureText` or any text measurement helper;
`Intl.RelativeTimeFormat`; `Intl.PluralRules`; `navigator.userAgentData`; `nanoid`, `uuid` or any id
counter; and `useLocalStorage`, `useInterval`, `useRaf`, `useEvent`, `useLatest`, `useMountedRef` or an
exported `sleep`. There is no truncation helper because truncation is done in CSS.

Most of those are gaps. Some are decisions, and filling them would be a regression.

**No `cx`, and none should be added.** There is no `cx`, no `clsx`, no `classNames` and no such
dependency in any of the four apps, and not one call site concatenates class names. That is not an
oversight, it is the architecture: all four style off data attributes, so a component's variant state is
`data-phase`, `data-open`, `data-compact` or `data-selected`, read by the stylesheet, while the class
name stays constant. A shared `cx` would work against that directly. It would make conditional classes
cheap, conditional classes would start appearing beside the attributes, and one component's styling
would then live in two mechanisms at once. The right shared helper for variant state is a
props-to-attributes mapping in `@margin/ui`, not a string joiner here.

Also deliberately not shared: `width.ts`, where only the persistence overlaps and `rootSetting` covers
it; calendar's `Intl`-free day and month arrays; the `mockIpc` bodies, which are per-app by definition;
the popover placement clamp, duplicated across four apps but with genuinely different anchoring rules,
which makes it the lowest confidence item in the audit; and id generation, which is margin-only because
calendar, editor and mail all take ids from Rust.

## Per app, in order

**calendar.** Add `"margin-shared"` to `package.json` and import the shared tokens in
`src/styles/tokens.css`; nothing else starts until this lands. Delete the empty `src/ipc/` directory.
Shim `escape.ts`, `useMedia.ts`, `store/useOverlays.ts` and `store/useToast.ts`. Adopt `platform.ts`
and the shared `call<T>`. Fix `normalizeCombo` by adopting mail's registry, which is also where its
context stack stops flattening to `global`. Give up `keys/updates.ts` to the shared updater but keep
`packagedBy` and `updateHint` as the option every app now gets. Move its date maths and `useClock` into
the package and re-export; its `EventDetailsModel.ts` clamp becomes the shared one.

**editor.** Shim `escape.ts` and `useMedia.ts`. Adopt `platform.ts`, the shared `call<T>` (gaining the
`log_note` write it does not have), `createOverlays`, and mail's toast store with its `seq`, which
fixes its dwell timer. Hand over `theme.ts`, `theme.test.ts`, `menu.ts`, `positions.ts`,
`components/Outline.tsx:79-89` and the update store and driver, and take back nine call sites' worth of
guarded storage as `makeStorage`. Replace `keys/` with the shared registry parameterised on its
`KeyContext`, keeping its `onCommand` fan-out. Its eight `aria-modal` dialogs get the trap through
`@margin/ui`.

**mail.** Fix the duplicate `marginmail-theme` constant and add the `packagedBy` guard first, before
either extraction touches those files. Hand over `escape.ts` (header included), `call<T>`,
`onAppEvent`, the toast store, the keyboard registry, `store/useOverlays.ts`, `screens/format.ts`'s
`space()` and its date formatters. Adopt editor's theme, which turns its fake System into a real
subscription and deletes `forgetThemeChoice`; adopt `rootSetting` for `pane.ts` and `appearance.ts`;
adopt the shared updater and delete both inline copies of `checkForUpdates`. Adopt `useCompact`, which
it does not have.

**margin.** Guard `project.ts:19` and tidy `library.ts:31` onto `parseJson` first. Add the shared
`call<T>`, renaming `isDesktop` to `isTauri` at the three call sites, and gain a log file entry for
every IPC failure for the first time. Hand over `focus.ts`, `time.ts`'s `relativeTime` and its
`plural`, and delete the second relative-time copy at `backup.ts:61`. Shim `escape.ts`; replace its
bare `useCompact` with calendar's, which writes `data-compact`. Adopt `platform.ts`, which makes its
five `e.metaKey` tests work off a Mac. Adopt `createOverlays` and the toast store when it takes the
primitives. Defer the keyboard registry until it has a palette and a shortcut sheet for the generated
output to land in; until then the four competing mechanisms stay, which is the one place in this plan
where an app knowingly keeps the worse thing.
