# Non-component TypeScript across the four apps

Scope: hooks, utilities, stores, the IPC layer. Non-test `.ts`: margin 4061 lines, margin-calendar 3811,
margin-docs 16868, margin-mail 11429. Shorthand: **M** margin, **C** margin-calendar, **D** margin-docs,
**X** margin-mail.

## The shared package already exists

`margin-shared` is at `/Users/pj/Workspace/projects/python/margin/shared`: a `file:` dependency of M, D and X,
exporting `.`, `./fonts`, `./icons` and two stylesheets, 299 lines of source today.

**C is not wired to it at all**, in TypeScript or CSS. It has no `margin-shared` in `package.json` and its
`src/styles/tokens.css` does not `@import "margin-shared/css/tokens.css"` the way the other three do. That
dependency line is the prerequisite for everything here.

**The extraction pattern is already proven.** `M/src/model/fonts.ts` (41 lines) and `D/src/model/fonts.ts`
(40) are re-export shims: pull the catalogue from `margin-shared/fonts`, declare one app-local alias
(`BookFonts` vs `DocumentFonts`) so call sites keep the app's own noun. Copy that shape.

No app uses tsconfig path aliases, so nothing needs build config beyond the dependency. All three vitest apps
run `environment: "node"`, so shared hooks need the `typeof window` guards D already writes, or
`vi.stubGlobal` as in `D/src/width.test.ts:24`. No app has a `utils/`, `lib/`, `helpers/` or `hooks/`
directory: everything is either a single-purpose top-level module or defined inline atop the one component
that needs it.

## Byte-identical today

**`src/escape.ts`**, all four. 36 lines in M, C and D, all three md5 `3b1f67d691647be7d61a23a5acd96a7b`. X's
is 40 lines, differing only by a four-line header; strip comments and all four hash identically
(`9c8530a09b35e5d697bb2b95674a2e73`). Signature `useEscapeLayer(active: boolean, onEscape: () => void): void`,
a module-level stack of Escape handlers behind one lazily-bound capture-phase listener. Move verbatim, keeping
X's header. 144 duplicated lines, zero risk. All three keyboard registries already defer to it by name rather
than handling Escape themselves, and its `const latest = useRef(onEscape); latest.current = onEscape;` is an
inline `useLatest` that falls out of the extraction for free.
**`useMediaQuery`**, all four `src/useMedia.ts`, byte-identical (`c3499ac5ee3d1a12b138d71b2cc787ed`).
**`usePhone` and `useTouch`**, C/D/X, byte-identical bodies (`d565f08a...`, `f4285e4c...`), with `PHONE_QUERY
= "(max-width: 640px)"` and `TOUCH_QUERY = "(pointer: coarse)"`; the only difference between the three files
is prose describing each app's layout. M has neither, only a stale `useCompact` on a 899px query that D
deleted when it added the phone/touch pair.

**`src/theme.ts`**, M/C/X. Strip comments, normalise the key, all three hash identically
(`ab9c99ff29565c686028e16960274d9d`); the literal M-to-C diff is one line. **`src/store/useTheme.ts`**, M and
C, byte-identical (`b48a3bccfadd21b9bb4efa91eef5041c`, 16 lines); X's 17 differ only by extracting a
`set(theme)` action. **`src/store/useToast.ts`**, C and D, byte-identical (`dfc90357bbcfbbfe71ffb8ba0e680c00`,
15 lines). M has no toast.

**The `src/ipc.ts` preamble.** Lines 1 to 41 of C's and D's are byte-identical
(`afde11927f75a3a81b2b459658c5b7be`), doc comments included: header, `isTauri`, `isMobileOs` with its iPadOS
carve-out, `isDesktop`, `isMacDesktop`, `live()`. So is the body of `call<T>`
(`b4259488afdd6a0f154b8e86eae1ff9c`). X has the same code with two comments abridged, plus one real addition
at `X/src/ipc.ts:752-758`: it logs a failed command to Rust before rethrowing, skipping `log_note` itself to
avoid a loop.

**`src/store/useOverlays.ts`**, C (64 lines) and X (72). Diffed with comments stripped, the entire difference
is the `Overlay` union and one prettier reflow of `push`; all seven actions are character-identical. X's
header: "Ported from the calendar's store of the same name."

**The keys platform block**, C/D/X, not one character differing, at `C:76-87`, `D:206-217`, `X:521-532`. Also
byte-identical across those three: the `NAMED` glyph map, `commandMatches`, `isTyping`, and
`pushContext`/`useKeyContext`.

```ts
const isMac = typeof navigator !== "undefined" && /mac|iphone|ipad/i.test(navigator.userAgent ?? "");
export const PRIMARY_LABEL = isMac ? "⌘" : "Ctrl+";
export const primaryHeld = (e: { metaKey: boolean; ctrlKey: boolean }): boolean =>
  isMac ? e.metaKey : e.ctrlKey;
export const secondaryHeld = (e: { metaKey: boolean; ctrlKey: boolean }): boolean =>
  isMac ? e.ctrlKey : e.metaKey;
```

## The three cleanest lifts

**`createOverlays<T>()`.** Nothing in the body knows what an overlay is. Each app writes `export const
useOverlays = createOverlays<Overlay>();` and keeps its own union. About 130 lines at zero behavioural risk. M
and D have no overlay store but both have palettes and dialogs and would adopt it.

```ts
export interface OverlayState<T extends string> {
  open: T | null;
  trail: T[];
  show: (overlay: T) => void;
  push: (overlay: T) => void;
  back: () => void;
  reachedFrom: (previous: T) => void;
  toggle: (overlay: T) => void;
  close: () => void;
}
export function createOverlays<T extends string>(): UseBoundStore<StoreApi<OverlayState<T>>>;
```

**`onAppEvent`.** Nobody wraps Tauri's `listen`, and it shows. X is the exception, at `X/src/App.tsx:69-77`,
signature `onAppEvent<T>(name: string, handler: (payload: T) => void): () => void`. It returns a synchronous
unsubscribe, so each effect is one line, and it falls back to `window.addEventListener` for a `CustomEvent` of
the same name outside Tauri, which is what lets Playwright drive the connect flow in a browser. C hand-rolls a
four-`.then(stop => stop())` teardown at `C/src/App.tsx:52-86`; D uses a third pattern, a module exporting
`startWorkspaceEvents(): () => void` that the shell mounts (`D/src/workspace.ts:366-378`).
The event names are already common property: `menu-action` in all four, `auth`, `sync-progress` and
`store-changed` in C and X, `pdf-warnings` in M and D. Four apps, the same names, three subscription
mechanics. Lift X's nine lines; keep D's "module exports `start*()`" convention on top.

**`useToast` and `notify`.** X's 34 lines beat the byte-identical 15 in C and D. Strict superset: an optional
`ToastAction { label, keycap?, run }` for undo, and a `seq` counter bumped on every notice. That counter is a
real fix. Auto-dismiss lives in the component in all three, and C's and D's dismiss effects omit a nonce from
the dep array, so notifying the same string twice does not restart the countdown; the second toast inherits
the remainder of the first one's timer. The dwell times also differ for no reason: 5000ms, 4200ms, 6000ms.
M has no toast, only a private `notify` at `M/src/store/useBackup.ts:45` writing into `useBook`'s notice field.

## Theme: margin-docs wins, and it is not close

`D/src/theme.ts` is 167 lines against 16, and every extra line earns it: a seven-palette table with a `scheme`
per row; a real tri-state `ThemeChoice = Theme | "system"` with a remembered light/dark pair;
`storedPreference()` dropping a stored id that no longer names a theme (without it, a palette retired between
releases leaves the root with a `data-theme` no stylesheet answers, which is not one broken colour but all of
them); `watchSystemScheme()`, the only live `prefers-color-scheme` subscription in any app;
`saved()`/`store()` guarding `localStorage` behind `typeof` and `try`/`catch`; and `theme.test.ts` (172
lines), the only theme test, holding the TypeScript table and the boot script's copy of it to one table.

X fakes a tri-state at the UI layer: "System" means deleting the stored key (`forgetThemeChoice()`,
`X/src/appearance.ts:41`), so it is a snapshot, not a subscription. Pick System at night and it stays dark
through the morning.

```ts
export interface ThemeConfig<T extends string> {
  themes: readonly ThemeInfo<T>[];
  keyPrefix: string;
  fallback: Record<Scheme, T>;
}
export function createThemeStore<T extends string>(config: ThemeConfig<T>): ...;
export function bootScript<T extends string>(config: ThemeConfig<T>): string;
```

Generating the boot script from the same config is the part to insist on. All four apps hand-maintain an
inline `<script>` in `index.html` re-reading the theme keys before the bundle exists, and D re-encodes its
whole palette-to-scheme table there in plain JS.

## The root-setting pattern, six times over

Bigger than theme. Read a boot attribute first because `index.html` already applied it, fall back to
`localStorage`, apply to the root, write back. `src/theme.ts` in all four on `data-theme`; `X/src/pane.ts` (20
lines) on `data-no-pane`, its header saying it is "exactly the shape `src/theme.ts` uses"; `D/src/width.ts`
(52) on `data-width`; `M/src/width.ts` (28) doing the same job through a `--measure` custom property;
`M/src/panes.ts` on `--pane-sidebar`/`--pane-dock` with its own private `clamp`; `X/src/appearance.ts` (43) on
`--font-ui`, `--font-heading` and `--body-size`.
The rule these obey is stated three times in three files: a store may not touch the DOM, and a layout fact the
stylesheet needs on first paint has to be an attribute, not a class on a component. Write it down once.
Key naming is already strict: every key in every app is `<slug>-<setting>`, slugs `margin-`, `margincal-`,
`margindocs-`, `marginmail-`. Counted: 8 keys in M, 6 in C, 17 in D, 9 in X. That prefix is the only per-app
parameter a storage helper needs.

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

D has written the guarded accessor pair nine times inside one app: `theme.ts:105`,
`store/useUpdate.ts:64,73,85`, `store/useProofing.ts:102,111`, `store/useDocumentFonts.ts:58,77`,
`workspace.ts:60,70`, `width.ts:33`, with the catch comment ("A webview with storage denied still X, it just
forgets between launches") repeated four times with only the verb swapped. Adopting it also fixes unguarded
reads in `M/theme.ts:8`, `C/time.ts:16`, `C/store/useCalendarView.ts:13`, `X/theme.ts:13`, `X/pane.ts:14` and
`D/Outline.tsx:30`; several run during module initialisation and would take the whole app down on a throw in a
storage-denied webview, and C's is already why `components/overlayModel.test.ts:20-31` stubs a global.

Do not reach for zustand's `persist` middleware, nor a `useLocalStorage` hook: the boot script reads these
keys before any bundle exists, so the shapes must stay plain and hand-chosen. `X/pane.ts:1-8` and
`X/appearance.ts:1-7` both document the constraint. `width.ts` itself should not be shared either. M has four
named widths on a CSS variable, D five on an attribute plus command wiring; only the persistence overlaps, and
`rootSetting` covers it.

## The IPC layer

Uniform in three of four and worth stating as the target: a frozen `src/ipc.ts` holding DTOs mirroring
`src-tauri/src/dto.rs` plus `call<T>`, and thin per-domain modules in `src/api/` doing nothing but naming a
command, e.g. `export const labelsList = (accountId: string | null) => call<LabelInfo[]>("labels_list", {
accountId });`

C has 6 api modules and 41 lines, D 9 and 211, X 16 and 386. X's `ipc.ts` is 760 lines because it has the most
DTOs, not because it is structured differently. Only X states the rule explicitly ("Nothing outside src/api
may call `call` directly").

M is the outlier. No `src/api`, `invoke` imported directly in 7 files and called at 8 sites. Its 43-line
`ipc.ts` has no `isTauri`, no `live()`, no `isMacDesktop` and no dev-fixture branch, which is why it has no
browser dev harness either. **And a real bug**: `M/src/ipc.ts:3` names its constant `isDesktop` but computes
what C/D/X call `isTauri`. The same identifier means the opposite thing in different apps, and in M it gates
`runWritingTool`, `listSystemFonts` and `gdriveListBackups`. M is desktop-only today so it does not bite yet.

Shareable: the whole preamble plus `call<T>` as a factory, since the dynamic `import("./dev/mockIpc")` path is
necessarily per-app. X's version wins, a strict superset of the byte-identical C and D; the `log_note`
recursion guard is the detail nobody would re-derive. DTOs stay per-app. `C/src/ipc/` exists and is empty;
delete it.

## The dev harness

C, D and X each have `src/dev/fixture.ts` and `src/dev/mockIpc.ts`: 134/222 lines, 696/344, 1769/1572. Every
`mockCall` is `(command: string, args?: Record<string, unknown>) => Promise<T>`, a switch on `command` ending
in the same byte-identical `dev mock has no handler for ${command}` throw, but the bodies are entirely
app-specific. A `createMockIpc(handlers)` supplying the dispatch and default throw is marginal; the real
shared pieces are the mock branch inside `call<T>` and X's `onAppEvent` fallback, both covered above. M has
no dev harness and cannot be driven in a browser.

## Keyboard: three registries, one design, no chords

C, D and X each have `src/keys` with `bindings.ts`, `keymap.ts`, `commands.ts`, `menu.ts`. No file is
byte-identical, but the scaffolding is one module forked three ways and several blocks inside are identical
(listed above).

**X's registry should win.** Three reasons that matter.

Its `resolve()` layers contexts instead of replacing them (`X/keys/keymap.ts:94-109`). A `screener` or `focus`
frame overrides only the keys it declares and leaves the base `view` keymap live underneath; only `overlay`
and `editor` shadow wholesale, via an explicit `SHADOWS_VIEW` list. C and D fall from the top frame straight
to `global`, which kills the base keymap the moment any non-overlay frame is pushed. X subsumes them; they
cannot express X.

Its `normalizeCombo` treats shift as a first-class modifier, so `Cmd+A` and `Cmd+Shift+A` are distinct. D
preserves case instead (`cmd+F` vs `cmd+f`), which works for letters on a US layout and breaks for punctuation
that only exists shifted. **C's lowercases behind a modifier, so `cmd+F` and `cmd+f` collide silently.** That
is a live bug.

Its `bindings.ts` does not import `commands.ts`, so the table, the sheet and the palette are testable without
booting stores or Tauri; in C and D the dependency runs bindings to commands to every store. Its `commands.ts`
is 75 lines with zero store imports: a `Map<CommandId, Handler[]>`, `registerCommands` returning its own
teardown, last-registered wins, so a screen takes over a verb on mount and hands it back. C's and D's are
static tables reaching into the whole app (224 and 433 lines). D is halfway there with its `onCommand`
fan-out, worth keeping alongside the stack for cases needing multiple listeners.

Take `menu.ts` from D instead: the same three lines everywhere, but only D exports its id list and tests that
every menu id names a real command.

**No app supports chords.** All three carry the same header line: "Nothing is chorded and nothing is modal.
Two keys never combine into a third meaning." No pending prefix, no timeout, no sequence buffer. A `g i`
binding is new work; X's `Map<string, Binding[]>` index and single `comboOf` extend to it most cleanly.

**M has no registry and should adopt one.** Four unrelated mechanisms: an `if`/`else if` chain on a
capture-phase window listener at `M/components/EditorView.tsx:167-192`, a second competing window listener for
Cmd+K in `M/editor/FloatingToolbar.tsx:57-70`, TipTap extension shortcuts in four files, and two hand-rolled
`menu-action` chains in `App.tsx:25-35` and `EditorView.tsx:199-206` unaware of each other.

M has no platform detection at all: `grep navigator.` over its `src` returns two clipboard calls. The five
`e.metaKey` tests in `EditorView.tsx` are macOS-only and silently dead on Linux and Windows; `primaryHeld`
fixes that for free. The blocker is that M has no palette and no shortcut sheet, so the "generated, never
maintained" payoff has nowhere to land yet.

Stays per-app: `BINDINGS` (20 rows, 20, ~70), the `CommandId` union, `GROUPS`, `MENU_IDS`, the command
implementations, and app-specific `KeyContext` members. `KeyContext` has to become a type parameter and
`SHADOWS_VIEW` an app-supplied list. The two `isMac` regexes differ by design and confusingly little:
`keys/bindings.ts` uses `/mac|iphone|ipad/i`, `ipc.ts` uses `/mac/i` gated on `isDesktop`, both correct for
their purpose. One `margin-shared/platform` should export both, plus `isTauri`, `isDesktop`, `isMacDesktop`,
`live`, `PRIMARY_LABEL`, `primaryHeld` and `secondaryHeld`, with an injectable user agent so the non-Mac
branch is finally testable; no test exercises it today.

## Stores: the conventions are already uniform

All 42 stores checked. Every one uses the plain `create<State>((set, get) => ({ ... }))`. Not one uses curried
`create<T>()(...)`. **No middleware anywhere**: zero hits for `persist`, `subscribeWithSelector`, `immer`,
`devtools`, `useShallow` or `zustand/shallow` in any app. Nothing is imported from `zustand` except `create`.

Selectors are uniformly inline, `(s) => s.field`, one hook call per field rather than one destructured object.
No exported selector functions exist in any store directory in any app. That is why no shallow comparator is
needed: every subscription is to a primitive or a stable reference. About 620 inline selector sites and 676
`getState()` calls.

Two conventions worth writing down because they are universal and currently retyped everywhere: `if (!live())
return;` as the first line of every backend-touching action, about 40 places; and `error: String(e)` in state,
never `e.message`.

The async action shape repeats about 100 times: optimistic set, try, replace with the server's answer, catch,
roll back and a `Could not ...` toast. Counts: M 1, C 10, D 33, X 53. **I would not abstract it**: the bodies
are five lines and each message is bespoke. Share the `Phase` union and a `describe(e)` helper only.

`useSearch`, `useAccounts` and `useSync` look shareable by name and are not. The three

`useSearch` stores do three different jobs. `useAccounts` in C and X shares a real idea, an OAuth consent
promise bridged over a Tauri event with the resolver stashed in state, but it is on its third hand-copy
(`M/store/useBackup.ts:70` to C to X) and is ~60% app-specific. Share the bridge, not the store, plus
`openAuthUrl`/`copyAuthUrl`, character-identical in both and pure utility.

```ts
export function deferred<T>(): { promise: Promise<T>; settle: (value: T) => void };
export function latestOnly<A extends unknown[], R>(fn: (...a: A) => Promise<R>): (...a: A) => Promise<R | undefined>;
```

`latestOnly` covers the stale-response race both search stores solve differently: D with module-level
monotonic counters (`useSearch.ts:20-21`), X by re-comparing the query string (`useSearch.ts:108`). D's is
more general; X's breaks if two callers share a query.

X's `useSync` is the best sync store, because ~160 of its 210 lines are pure exported functions over
`SyncStatus[]` rather than store code, with a 9.6KB test behind them. C keeps the same dedupe logic as an
effect-local closure variable in `App.tsx`.

## Cross-cutting utilities

**Confirmed absent from all four**: any debounce or throttle utility; deep equality; a `safeParse`/`tryParse`
wrapper; class-name joining (no `cx`, no `clsx`, no `classNames`, and no such dependency); `measureText` or
any text-measurement helper; `Intl.RelativeTimeFormat`; `Intl.PluralRules`; `navigator.userAgentData`;
`nanoid`/`uuid` or any id counter; and `useLocalStorage`, `useInterval`, `useRaf`, `useEvent`, `useLatest`,
`useMountedRef` or an exported `sleep`. No truncation helper; truncation is done in CSS.

The class-name negative is a deliberate architectural choice, not an oversight: all four apps style off data
attributes, so no app ever concatenates class names. Do not introduce `cx`.
**debounce.** Written longhand at 15 sites across all four. The React-effect variant is the same five lines in
at least six places and is worth one `useDebounced<T>(value, ms)`; D's `QuickOpen.tsx:88` and
`FindInFiles.tsx:70` are already identical and name the constant `DEBOUNCE_MS`. Leave the class-field and
module-level timers alone; their cancel/flush semantics are the point.

**`clamp`: 6 definitions plus 12 inline sites, zero shared.** `C/grid/fit.ts:89` and
`C/components/QuickCreateModel.ts:147` are the identical one-liner duplicated *within C*:

```ts
const clamp = (n: number, lo: number, hi: number) => (n < lo ? lo : n > hi ? hi : n);
```

`C/components/EventDetailsModel.ts:45` should win: it is the only one guarding an inverted range, which
matters because half the call sites pass `length - 1` as the high bound and that goes negative on an empty
list. `QuickCreateModel.ts:156,163,171` already works around exactly this at every call. Add a `clampIndex(i,
length)` for the ten index sites. `M/ExportPreview.tsx:92` and `D/ExportPreview.tsx:143` are the
byte-identical zoom clamp.

**Byte size: four implementations, three spellings.** `D/UpdateDialog.tsx:21` says `kB`, `D/FileViewer.tsx:43`
says `bytes` and `KB` with one decimal, `X/screens/format.ts:52` says `B`/`KB`/`MB` with rounding,
`X/screens/Settings.tsx:940` adds GB and adaptive precision. 1536 bytes renders as "1.5 KB", "2 kB" and "2 KB"
in one product family. X's `space()` wins: the only one reaching GB, and its `mb < 10 ? toFixed(1) :
Math.round(mb)` rule actually solves what D's UpdateDialog comment states ("so the number under the bar stops
twitching"), by significant figures rather than fixed decimals.

**Relative time: three hand-rolled ladders, no two agreeing.** `M/src/time.ts:1` is the only standalone
module; `M/src/backup.ts:61` is a second copy in the same app with the prefix baked into every branch;
`D/components/Settings.tsx:45` is a third, inline, with "Checked" hardcoded throughout. They diverge in
behaviour, not just text: M floors, D rounds; M's cutover is 7 days, D's 30. D's rounding makes 89 seconds
"just now" and 91 seconds "2 minutes ago". M's wins: the only one guarding clock skew with `Math.max(0, now -
ts)`, and flooring is right for "how long ago". Callers compose their own prefix, deleting `backup.ts:61` and
reducing `Settings.tsx:45` to one line. X has no "X ago" at all; `rowTime`/`messageTime`
(`screens/format.ts:34-49`) go straight to calendar-day buckets and are the only implementation counting by
calendar day via local midnight rather than elapsed hours, which is the difference between "Yesterday" being
right and being off by a few hours every evening. Keep both.

**`useClock` is the missing half.** `C/src/useClock.ts` (`useTick<T>`, `useMinuteTick`, `useHourStart`)
re-schedules against wall-clock rather than a `setInterval` and treats `visibilitychange` and `focus` as
ticks, because a timer's deadline is measured in time the machine spent awake. C is its only consumer today,
but M's `relativeTime` callers both take a `now` prop that has to come from somewhere, and X's row formatters
default to `Date.now()` at call time and go stale on a window left open overnight. Ship it beside
`relativeTime`.

**`Intl.DateTimeFormat`: no shared factory.** The same `{ day: "numeric", month: "short" }` formatter is
constructed in 6 X files and the `hourCycle: "h23"` clock in 3. M and D use bare `toLocaleDateString()`. A
memoised `dateFormat(options)` collapses the lot; X's cached-module-constant style is the right pattern.

C's `src/time.ts` (136 lines) avoids `Intl` entirely with hand-written day and month arrays, a deliberate
product decision for a calendar grid that should stay. Its `startOfDay`, `addDays`, `isSameDay`, `toDateOnly`,
`parseDateOnly` are reusable date maths and should move; `startOfDay` is already written three times across C
and X (`C/time.ts:23`, `X/screens/format.ts:20`, `X/SnoozePicker.tsx:72`).

**Focus management: one app has it, three do not.** `python/margin/src/focus.ts` (56 lines) is a proper trap:
a `FOCUSABLE` selector, Tab wrapping both directions, opener restore on teardown, and a module-level
`focusTrapped()` so the global key handler can stand down. Five call sites in M.

```ts
export function useFocusTrap(ref: RefObject<HTMLElement | null>, active = true): void;
export function focusTrapped(): boolean;
```

C has autofocus only, no trap and no restore (`components/overlayShell.tsx:51-56`). D and X have neither: both
ship `role="dialog" aria-modal="true"` markup with imperative `.focus()` on mount and no trap behind it.
**This is the one item where sharing fixes an accessibility gap in three apps rather than removing
duplication.** The wrinkle: M's `focusTrapped()` and the other apps' context stack are two mechanisms for one
idea. Either the trap pushes an `overlay` frame, or the shared keymap takes a "something external owns the
keyboard" predicate.

**Scroll position restore: near-identical in M and D, and D says so.** `python/margin/src/editor/positions.ts`
(64 lines) and `rust/margin-editor/src/editor/positions.ts` (52) share an identical `{ from, to, scroll }`
interface, the same guarded `readAll`/`write`, and the same load/save API. D's header names the relationship:
M keys by book then chapter, D by absolute path, "and that path is the whole key". D adds an LRU trim at
`LIMIT = 200`; M's map grows unbounded. D wins, with a flat string key M composes as `${bookId}/${chapterId}`.

`scrollIntoView` is raw at 8 sites, never wrapped; `M/editor/search.ts:98` and `D/editor/search.ts:111` are
byte-identical. Worth extracting alongside it is `D/components/Outline.tsx:79-89`, the only implementation
avoiding `scrollIntoView` scrolling every ancestor, already duplicated once inside D at
`editor/linkPicker.ts:572`.

**JSON.** 10 `JSON.parse` sites, 8 guarded inline, 2 not: `M/library.ts:31` and `M/project.ts:19` parse a file
read off disk with no catch, so a truncated or hand-edited project file throws unhandled. Fold the guarded
ones into `readJson<T>(key, fallback)`; the file reads want `parseJson<T>(text): T | null`. D's `const parsed:
unknown = JSON.parse(...)` then narrow is the right discipline; M and X cast straight to the target type.
**`plural`** is the same function under different names in `M/store/useBackup.ts:56` and
`D/linkRewrite.ts:694`, plus 10 open-coded `${n === 1 ? "" : "s"}` sites including two byte-identical ones in
`X/screens/ReadingPane.tsx:235,502`. M's name wins.

**ResizeObserver**: 13 raw instantiations, no hook. `M/ExportPreview.tsx:64` and `D/ExportPreview.tsx:104` are
byte-identical, as are `M:238`/`D:339` and the `IntersectionObserver` at `M:286`/`D:395`. Any shared
`useResize` must carry the null guard `X/screens/MessageBody.tsx:207` documents and the others lack:
"`ResizeObserver.observe(null)` throws hard enough to take the screen with it". `getBoundingClientRect` has 32
inline sites, all positioning rather than text metrics; the popover-placement clamp is duplicated across four
apps but the anchoring rules differ meaningfully, so it is the lowest-confidence item here. Id generation is M
only, `crypto.randomUUID()` raw at 8 app-domain sites; C, D and X take ids from Rust, so there is nothing to
share.

## Updates: four apps, four answers

M has `src/updater.ts` (111 lines) plus `store/useUpdater.ts` (34), a `direct` vs `appstore` channel split,
and it holds the live `Update` resource handle in the zustand store. C has `keys/updates.ts` (41), toast-only,
"Ported from margin's `src/updater.ts`, minus its progress dialog"; it is the only one of the four with a
`packagedBy()` guard, so a nix or homebrew install is told to update through its package manager instead of
self-updating. X has no update module at all: `checkForUpdates` is inline at `X/src/App.tsx:98-118`, with a
second copy in `screens/Settings.tsx`.

D has `src/update.ts` (171) plus `store/useUpdate.ts` (129), the best of the four: named phase transitions
rather than a generic `set(partial)`; `total: number | null` so a missing content-length draws an
indeterminate bar instead of 0% forever; version and notes as plain strings so no Rust handle sits in the
store; a launch delay and 24-hour interval for automatic checks; a flush of the pending save before
`relaunch()`; and a discriminator for "this dev build has no updater plugin" so that reads as a sentence
about the build. Take D's store and driver, fold in C's `packagedBy` guard as an option.

## Bugs found, worth fixing regardless of any extraction

- `marginmail-theme` is declared as two constants in two files, `X/src/theme.ts:8` and
  `X/src/appearance.ts:14`.
- X's `checkForUpdates` lacks C's `packagedBy()` guard, so a package-manager-installed build will try to
  self-update over a path it does not own.
- C's `normalizeCombo` lowercases the key behind a modifier, so `cmd+F` and `cmd+f` are one binding.
- `M/src/ipc.ts:3` names `isDesktop` what the other three call `isTauri`, and it gates three IPC calls.
- `M/library.ts:31` and `M/project.ts:19` parse a file off disk with no catch.

## Ranked

1. `escape.ts`. ~144 lines, byte-identical in four apps.
2. `createOverlays<T>()`. ~130 lines, code-identical today.
3. `platform.ts` (both halves). ~50 lines, byte-identical in three, fixes two M bugs.
4. `onAppEvent`. ~60 lines, better teardown, unlocks browser testing for two apps.
5. `useMedia.ts`. ~120 lines, byte-identical bodies in three apps.
6. `useToast` plus `notify`. ~60 lines, strict superset, fixes a timer bug in two.
7. `makeStorage` and `rootSetting`. ~120 lines, deletes nine copies in D, fixes six unguarded reads.
8. `call<T>` factory plus the `ipc.ts` preamble. ~50 lines.
9. Theme, from D, with a generated boot script. ~250 lines, needs a per-app table.
10. Small utilities: `clamp`, `fileSize`, `relativeTime` plus `useClock`, `plural`, `positions.ts`,
    `useDebounced`, `dateFormat`, `latestOnly`, `deferred`. Collectively ~250 lines and several
    user-visible inconsistencies.
11. `useFocusTrap`. Not a saving; an accessibility fix for three apps.
12. The keyboard registry, from X. Largest and highest-value, but needs `KeyContext` parameterised and
    `SHADOWS_VIEW` injected, and M needs a palette before it benefits.
13. The update store and driver. Four designs to reconcile; do it last.

Items 1 to 6 are mechanically identical across apps today and drop into the existing package with no behaviour
change. The one prerequisite is adding `margin-shared` to margin-calendar.

