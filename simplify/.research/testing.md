# Testing, fixtures and the dev harness across the four apps

Scope: vitest and playwright config, the `tests/` suites and their helpers, `src/dev/` (the
browser-against-fake-data harness), Rust fixtures and `#[cfg(test)]` conventions, the tsconfig
split. Not build tooling, not CI beyond how it invokes tests.

Paths: margin `/Users/pj/Workspace/projects/python/margin`, margin-calendar
`/Users/pj/Workspace/projects/python/margin-caledar`, margin-docs
`/Users/pj/Workspace/projects/rust/margin-editor`, margin-mail
`/Users/pj/Workspace/projects/rust/margin-mail`. Cites are relative to those roots.

## Findings first

1. **margin has no tests of any kind.** Confirmed plainly: `package.json:5-12` has `dev`, `build`,
   `preview`, `tauri`, `dmg`, `fonts:sync`, `fonts:check` and nothing else, and neither `vitest` nor
   `@playwright/test` is a dependency. No `tests/`, no `src/dev/`, zero `*.test.ts` under `src/`,
   zero `#[cfg(test)]` and zero `#[test]` under `src-tauri/src/`. Its `vite.config.ts` imports from
   `vite`, not `vitest/config`, so there is no `test` block to add to, and its `src/ipc.ts` (43
   lines) calls `invoke` directly behind an `isDesktop` early-return, so there is no seam a fixture
   could plug into. Bringing margin into a shared harness is not a config change, it is building the
   dev fixture it never had.
2. **The three playwright configs are one file with the port swapped.** Diffing them leaves the port
   (1430/1440/1450), two rewritten comments, and two real differences: margin-docs adds
   `globalSetup: "./tests/identity.ts"` and drops `timezoneId`. Everything else is byte identical.
3. **margin-docs is the only app that checks the dev server is serving its own checkout**, and all
   three set `reuseExistingServer: true` on a fixed port. `tests/identity.ts` (106 lines) fetches
   five source files over `?raw` and byte-compares them against disk. Its own header says the four
   suites beside it "were happy to pass against anybody's copy". margin-calendar and margin-mail
   still are.
4. **The dev-mode invoke stub is the thing worth sharing and the three apps solved it three
   different ways.** All three branch identically in `src/ipc.ts`, but margin-calendar fakes no
   events at all, margin-mail fakes them as window `CustomEvent`s with an app-side bridge, and
   margin-docs hand-rolls 97 lines of `__TAURI_INTERNALS__` inside a test helper so the real
   `@tauri-apps/api` event plugin works in a plain tab. The third is the correct one and it is the
   one that is not reusable, because it lives in `tests/disk.ts` rather than in `src/dev/`.
5. **margin-docs has no shared spec helper at all.** 19 of 19 spec files define their own
   `async function open()` and inline the same `margindocs-recents` localStorage seed.
   margin-calendar and margin-mail both have `tests/app.ts`, and 33 of their 34 spec files import
   `openApp` from it. Between those two, `contrastOf` (60 lines), `clockAt`, `MIDDAY`, `settle`,
   `box` and `openDialog` are code identical and differ only in doc comments.

## What each app has

| | margin | calendar | docs | mail |
| --- | --- | --- | --- | --- |
| `test` script | none | `vitest run` | `vitest run` | `vitest run` |
| `test:ui` script | none | `playwright test` | `playwright test` | `playwright test` |
| `vitest` dep | no | 3.2.4 | 3.2.4 | 3.2.4 |
| `@playwright/test` dep | no | 1.62.1 | 1.62.1 | 1.62.1 |
| `src/dev/` | absent | 356 lines | 1,040 lines | 3,341 lines |
| spec files / source-level tests | 0 | 10 / 131 | 19 / 116 | 24 / 252 |
| colocated `*.test.ts` / tests | 0 | 12 / 211 | 32 / 655 | 8 / 81 |
| Rust `#[test]` | 0 | 79 | 0 (all integration) | 531 |
| `src-tauri/tests/` | no | no | 8 files + support | no |
| `src-tauri/fixtures/` | no | no | no | 76 files |

Source-level counts are `test(` and `it(` at file scope; several files wrap tests in a
`for (const theme of ["light","dark"])` loop, so the run counts are higher. CI runs `pnpm test` and
`cargo test` in all three (calendar `ci.yml:31,70`, docs `ci.yml:34,58,78`, mail `ci.yml:49,92`) and
Playwright nowhere. `just test` is `pnpm test` plus `cargo test` and `just test-ui` is
`pnpm test:ui`, the three justfiles agreeing line for line on both.

## playwright.config.ts

Shared by all three: `testDir: "./tests"`, `fullyParallel: true`, `retries: 0` (with the same
comment in each saying a test that only passes on the second go is lying), `reporter: [["list"]]`,
`outputDir: "node_modules/.cache/playwright"`, `timeout: 30_000`, `expect.timeout: 5_000`,
`baseURL` off the port, `viewport: {1440, 900}`, `deviceScaleFactor: 1`, `locale: "en-GB"`,
`trace: "retain-on-failure"`, `screenshot: "only-on-failure"`, a single project
`{ name: "chromium", use: { browserName: "chromium" } }` with an identical comment explaining why it
is not `devices["Desktop Chrome"]` (that device pins a Windows user agent and the keymap reads the
platform off it), and a webServer block of `pnpm dev`, `reuseExistingServer: true`,
`timeout: 60_000`, `stdout: "ignore"`, `stderr: "pipe"`.

Differences, in full: port 1430 / 1440 / 1450; `timezoneId: "Asia/Kolkata"` in calendar
(`playwright.config.ts:34`) and mail (`:31`) but not docs, because both anchor their fixture to the
browser's local day; `globalSetup` in docs only (`:14`).

No app configures `toHaveScreenshot`, `snapshotDir` or any visual-regression comparison, and there
are zero snapshot baselines in the three. Every screenshot is a named PNG for a person or the docs.

## The tests directories

margin-calendar, 10 specs plus `app.ts` (370 lines) and `tsconfig.json`. Geometry and layout
(`grid` 26, `compact` 5, `views` 8), input (`keyboard` 14, `interaction` 12, `touch` 22), form
factor (`phone` 10), design-system (`legibility` 10), overlays (7), regression (`bugs` 17). The two
phone files use `test.use({ viewport, hasTouch, isMobile })` at file scope (`touch.spec.ts:19`,
`phone.spec.ts:16`) rather than a Playwright project.

margin-docs, 19 specs plus `caret.ts` (134), `disk.ts` (144), `saving.ts` (52), `identity.ts` (106).
Six of the 19 are `_audit*.spec.ts`, headed "Temporary exploration harness. Deleted once the
findings are written down" (`_audit.spec.ts:1`) and still present. The rest are bytes-on-disk claims
(`bytes` 11, `color` 7, `export-writes-only-the-pdf` 2, `external-changes` 8), editing (`blocks`,
`headings`, `tables`, `clipboard`, `shortcuts`), and smoke (9).

margin-mail, 24 specs plus `app.ts` (401 lines) and `tsconfig.json`. Roughly one spec per screen,
plus `kit.spec.ts` (design system), `keyboard.spec.ts`, `guide.spec.ts` and `guide-shots.spec.ts`
(asset generation, not assertion).

### The helpers each invented

`tests/app.ts` in calendar and mail is the same file with different measurement functions bolted on.
Identical modulo doc comments:

- `clockAt(hour, minute)` and `MIDDAY` (cal `:41-53`, mail `:46-58`), pinning the clock to the
  current day at a fixed hour in Asia/Kolkata.
- `openApp(page, options)` (cal `:61-90`, mail `:66-90`). Same `addInitScript` body, same
  `__test-seeded` sentinel so a reload is not silently reset, same `page.clock.setFixedTime`. Only
  the seed keys differ.
- `settle(page)`, two `requestAnimationFrame`s (cal `:105-112`, mail `:99-106`, third copy in docs
  `tests/caret.ts:46-53`), `box(target)` (cal `:114-124`, mail `:108-118`), and `openDialog(page)`
  (cal `:288-293`, mail `:188-193`).
- `contrastOf(page, selector)` (cal `:306-366`, mail `:219-278`), which composites every translucent
  background between the element and the page through a 1x1 canvas so `oklch()` and `color-mix()`
  do not read as transparent.

App-specific and correctly so: calendar's `gridFit`, `axis`, `blocks`, `headerDates`, `hourY`,
`columnX`, `drag`; mail's `rows`, `groups`, `paneMessages`, `paletteRows`, `actionBar`,
`checkedRows`, `bodyText`, `toast`, `listScroll`, `place`, `openRow`.

Two mail helpers are generic and belong in a shared package despite being written for mail:
`token(page, name)` (`app.ts:180-185`), reading a CSS custom property off the root, and
`failCommands(page, commands)` (`app.ts:343-360`), which uses `page.route` to answer the request for
`/src/dev/mockIpc.ts` with a shim that forwards to the real module (`?real`) and rejects the named
commands. That is the only way to test a failure path when the backend is in the page rather than on
the wire, and it is 18 lines that would work unchanged in any of the four.

margin-docs' helpers have no sibling: `putCaret`/`caretIsIn` (`caret.ts`), whose 28-line header
documents why nothing in the suite presses End to move a caret; `watchDirty`/`dirtyWasShown`
(`saving.ts`), a MutationObserver installed before typing so the 500ms autosave cannot be raced; and
`installTauriShim`/`change`/`ask` (`disk.ts`), covered below.

### Screenshot policy

`page.screenshot` appears 33 times across 16 mail specs, 7 times across 4 docs specs (all in
`_audit*`), and never in calendar. Mail writes into `screenshots/`, which is gitignored
(`.gitignore:41`), so an ordinary run leaves the tree clean; the 10 pictures that ship inside the
bundle go to the committed `public/guide/` and are gated by
`test.skip(() => !process.env.GUIDE_SHOTS)` (`guide-shots.spec.ts:19-22`) behind
`just guide-shots` (`justfile:28-30`). That two-tier rule is right and only mail has it.

### Design-system assertions

`mail/tests/kit.spec.ts` (132 lines) is the only design-system suite. It renders a `#/kit` route in
both palettes and shoots it, asserts fixed geometry read from custom properties (`--list-w` is
`420px`, a row is 46px, an avatar 30x30), then runs three static source scans with `node:fs`: no hex
literal in any stylesheet under `src/ui` or `src/screens`, every `<input>` carries `NO_AUTOFILL` or
an explicit `autoComplete`, and no file says "keychain".

calendar's equivalent is runtime rather than static: `legibility.spec.ts` measures contrast, checks
no block reads "Untitled" and checks every block has a non-empty accessible name, over both themes.
docs' equivalent is a vitest, not a spec: `src/theme.test.ts` reads three stylesheets (including
`node_modules/margin-shared/css/tokens.css`, `:18-22`) plus `index.html`'s pre-bundle boot script
and asserts all of them declare the same variable set, because a missing dark variable falls back
silently to the warm light value in `:root`. Nothing anywhere asserts anything about icons.

## src/dev: the invoke stub

All three apps switch dev mode on the same way, in `src/ipc.ts`:

```ts
export function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (import.meta.env.DEV && !isTauri) {
    return import("./dev/mockIpc").then((m) => m.mockCall<T>(command, args));
  }
  return invoke<T>(command, args);
}
```

calendar `ipc.ts:192-197`, docs `ipc.ts:288-292`, mail `ipc.ts:748-752`, byte identical bar the
comment. `isTauri` is `"__TAURI_INTERNALS__" in window`, computed once at import time, and all three
derive `isDesktop`, `isMacDesktop` and `live()` from it with the same comments verbatim (calendar
`:9-40`, docs `:9-40`, mail `:10-34`). There is no environment variable and no dev-only build flag:
the switch is "DEV build with no Tauri bridge", so `pnpm dev` in a browser and Playwright get the
fixture and the packaged app cannot.

`mockCall` is a `switch (command)` in all three, returning `as unknown as T` at every arm and
throwing on an unknown command (calendar `mockIpc.ts:131-132`, "dev mock has no handler for"). There
is no type link between a command name, its arguments and its return; calendar casts args to
`Record<string, never>` (`:41`) and then to the real type per field. Command counts: calendar 12
arms, docs 40, mail roughly 180. State is a mutable copy of the fixture taken at module load, so
writes persist for the session and a reload resets (calendar `:11-14`, docs `:48-49`, mail `:72-84`).

### Fixtures

`src/dev/fixture.ts` in each. calendar (222 lines) exports `devAccounts`, `devCalendars` and
`devInstances(from, to)`, generated from a seed table and anchored to the current week; its header
records that it is modelled on a real sync of 12,067 events where 90% came back with no summary.
docs (344 lines) exports `devRoots`, `devEntries` (an in-memory folder with real base64 PNG, SVG and
PDF bytes at `:208-273`) and path utilities the mock and the app both use. mail (1,572 lines)
exports 20 symbols, 13 data tables (`devAccounts` through `devSyncStatus`) and 6 functions
(`devDiscover`, `devCert`, `devFiles`, `devContacts`, `groupOf`, `categoryOf`).

A fourth fixture source in docs has no sibling: `src/markdown/corpus/`, loaded through
`import.meta.glob("./*/*.md", { query: "?raw", eager: true })` at `corpus/load.ts:20`. Its header
records the bug that motivated the glob, that naming folders explicitly left twenty adversarial
files inside the corpus but outside every gate reading it. That is the "add a file, get a test"
pattern and it is worth generalising.

### Dev flags

localStorage, read defensively inside try/catch. calendar has one, `margincal-dev-empty`
(`mockIpc.ts:32-38`); docs has one, `margindocs-dev-no-writing-tools` (`:246-252`); mail has eight,
`marginmail-dev-` plus `empty`, `crowd`, `notify`, `imap`, `bridge`, `pending`, `hydrate-fails`,
`sync-fails`, read through `firstRun()` (`:220-230`) and a generic `flagged(key)` (`:262-268`). The
naming is uniform (`<app>-dev-<thing>`) and the reader is the same eight lines three times over.

### Faking events, three ways

This is where the three diverge and where the shared design has to be decided.

calendar does not fake events at all: `App.tsx:52` is `if (!isTauri) return;` before every `listen`,
so in a browser `menu-action`, `auth`, `sync-progress` and `store-changed` never arrive, and
anything they drive is unreachable from the Playwright suite.

mail dispatches window `CustomEvent`s under the same names from inside the mock
(`mockIpc.ts:237-241`), with an optional delay so a state that would otherwise last one frame is
observable (`narrateFirstSync` at `:637-651` walks a five-step sync over 1,250ms). The app side is
`onAppEvent` in `App.tsx:69-77`, seven lines picking `listen` or `window.addEventListener` off
`isTauri`. Cheapest correct answer, and confined to mail.

docs does neither. Its mock exports `external` (`mockIpc.ts:619-696`), a second surface that mutates
the fixture the way another program would, behind the app's back, and returns the exact
`WatchEvent[]` the Rust watcher would have emitted; putting them on the bus is the caller's job.
That caller is `tests/disk.ts:37-133`, which installs a hand-rolled `__TAURI_INTERNALS__` at
document start: `invoke`, `transformCallback`, `unregisterCallback`, `runCallback`, the
`plugin:event|listen` and `|unlisten` commands, a listener map, `metadata`, `convertFileSrc` and
`__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener`. Its comment at `:29-36` says this is
deliberately not `@tauri-apps/api/mocks`, which cannot be reached from an init script, and
deliberately written to the contract rather than to convenience. `emit` returns a delivery count per
event so a test can tell a working subscription from a payload that fell on the floor (`:66-75`).
`external` also carries `pauseWrites`/`resumeWrites` (`:679-695`) so a save can be held and a buffer
kept dirty instead of racing the 500ms autosave.

The docs approach is the strictly better one: the app runs the real `@tauri-apps/api`, so the
`isTauri` branch that ships is the branch under test, where mail leaves `onAppEvent`'s Tauri arm
never exercised in a browser. But it is 97 lines in a test helper, so a person running `pnpm dev` by
hand gets no events at all.

## The Rust side

margin-mail `src-tauri/fixtures/` is 36 `.eml` files (real messages as they come off the wire, CRLF
throughout, half of them not UTF-8), 36 matching `golden/*.txt` and 4 `autoconfig/*.xml`, compiled
into the test binary by a `corpus!` macro in `src/fixtures.rs:10-19` that emits one `include_bytes!`
const per file plus an `all()` returning every pair, so a sweep over the corpus is one call.
`fixtures.rs:58-102` is itself a test: every fixture has a header/body break, no bare LF, a
plausible date and a parseable From. Nothing is generated at test time.

Alongside it, `src/provider/fake.rs` (612 lines, `#![cfg(test)]` at `:15`) is an in-memory mailbox
implementing the `Provider` trait: real ids, labels, dates and raw bytes, paging, a history log, and
scripted failures via `fail_next` and `withhold_body`. `provider/mod.rs:1-14` records that this is
the only reason the sync engine is testable without credentials.

Nothing comparable exists elsewhere. margin-docs builds its Rust fixture at runtime instead:
`src-tauri/tests/support/notes_repo.rs` (339 lines) creates a real git repository per test binary,
copies 12 documents out of `src/markdown/corpus/real` so there is one corpus and not two, and
generates 13,000 files under a vendored `node_modules` because several tests need a folder large
enough that skipping it beats walking it. `git status` is the oracle.

Conventions differ and both are defensible. calendar and mail put tests in `#[cfg(test)] mod tests`
beside the code, 6 files / 79 tests and 54 files / 531 tests; fifteen of mail's are large enough to
live in a sibling `<module>/tests.rs` (backup, clips, contacts, decisions, drafts, exports, invites,
notify, piles, screener, send, snooze, state, sync, unsubscribe). docs has zero `#[cfg(test)]` and
eight integration binaries totalling 4,210 lines, one needing `--test-threads=1` and run as its own
CI step (`ci.yml:78`). Eight mail tests carry `#[ignore]`, six of them "hits the network" in
`imap/discover.rs`.

## tsconfig

The app `tsconfig.json` is `"include": ["src"]` in all four, so `tests/` is excluded by omission
rather than by an `exclude` entry, and the colocated `src/**/*.test.ts` files sit inside the app's
own type check. `tests/tsconfig.json` exists in the three and is the same nine options each time
(ES2022, bundler resolution, strict, noUnusedLocals, noUnusedParameters, skipLibCheck, noEmit) with
`"include": [".", "../playwright.config.ts"]`. Mail's adds `"types": ["node"]`. Nothing runs `tsc -p
tests/tsconfig.json` in any script or CI job, so the spec files are type checked only by an editor.

Vitest config lives in `vite.config.ts` under `test:`, all three with
`include: ["src/**/*.test.ts"]` and `environment: "node"`. No jsdom, no happy-dom, no
`@testing-library/*` anywhere. margin-docs adds `maxWorkers: "50%"` and three 30-second timeouts
with a 20-line comment explaining that a CPU-blocking markdown sweep cannot answer vitest's
`onTaskUpdate` RPC while it runs.

## Colocated unit tests: what they are and are not

They test pure functions in a node environment. No component is rendered, no store is mounted
against a DOM, nothing goes near `mockIpc`. Calendar's convention is explicit: a component `X.tsx`
gets a sibling `XModel.ts` holding the decisions and `XModel.test.ts` tests that module.
`GridModel.test.ts` covers `busyHours`, `heldHours` and `bandAt` over synthetic `Placed` values with
`instance: {} as Instance`, because only the times matter; `EventDetailsModel.test.ts` covers
popover placement against a fixed `Bounds`; `overlayModel` and `QuickCreateModel` cover date
arithmetic and draft construction; `AgendaModel` covers grouping, gap labels and search matching.
The odd one out is `EventDetailsHtml.test.ts`, half readability and half hostile input, because the
description string comes off the wire from whoever created the event.

margin-mail's eight follow the same rule. `providers.test.ts` tests the routing decision for an
address twice over, by domain and by what discovery found; `useSync.test.ts` the two pure decisions
the header and toast make from a `SyncStatus`; `guide.test.ts` link-checks the article library
without rendering it; the other five are table checks.

margin-docs' two named ones are a different genre and the more interesting one. `theme.test.ts` and
`width.test.ts` both read source with `node:fs` and assert that two tables not written in the same
language agree: theme against three stylesheets and a boot script, width against the command table
and the subscription joining them. The other 30 are the markdown bridge and are app-specific.

None of them snapshot, mock `call()`, or configure coverage, and there is no property-based library
(though `typst.test.ts` and the five `adversarial*.test.ts` files are hand-rolled corpus sweeps,
which is the same idea).

## Known-failing specs

Nothing in any repo records a known failure. The only markers are `guide-shots.spec.ts:19` (an
intentional env gate, not a failure) and a `test.fail` at `external-changes.spec.ts:320`, kept so it
would turn red the day the defect was fixed. `margin-editor/docs/conventions.md:74` is the standing
rule: "Never weaken, skip or delete a test to reach green."

The four margin-mail browser failures recorded in session memory are written down nowhere in the
tree. If they are real that is the gap: either a per-app `tests/known-failures.md` or annotations on
the specs, because right now the only record is a chat log.

## What a shared harness package would contain

`margin-shared` already exists at `margin/shared` and is consumed by margin and margin-docs by
relative `file:` path, and by margin-mail. margin-calendar does not depend on it at all. Adding a
`./test` subpath export is the least-friction home; a separate package means a fourth `file:` edge.

**1. A playwright config factory.** Everything in the three configs bar the port is a default, so
the whole file becomes `export default marginPlaywrightConfig({ port: 1450, witnesses: ["src/ipc.ts",
"src/dev/mockIpc.ts", "src/dev/fixture.ts"] })`. `witnesses` turns the docs identity check on for
every app rather than one, with the factory supplying `globalSetup` so no app wires it. `timezoneId`
defaults to `Asia/Kolkata` and docs passes null. Saves roughly 45 lines per app and makes "all three
run at 1440x900 with retries 0" a fact rather than a coincidence.

**2. The dev backend, with a typed command registry.** Replace `switch (command)` with a table keyed
by command whose handlers are typed off the DTO types `src/ipc.ts` already declares:

```ts
export const backend = defineBackend({
  accounts_list: () => (firstRun() ? [] : accounts),
  thread_view: ({ key }: { key: string }): ThreadView => viewOf(byKey(key)),
});
export type Command = keyof typeof backend;
```

`defineBackend` returns `{ mockCall, has, commands }`, with `mockCall` throwing the existing "no
handler for" error on a miss. The win is that the arg cast at every arm and the `as unknown as T` at
every return both disappear, and a command in `dto.rs` with no handler becomes checkable rather than
a runtime surprise three screens later. Alongside it: `makeCall(loader)` producing the `call()` in
`src/ipc.ts`, already identical in three places, and `devFlag(name)` replacing the three copies of
the try/catch localStorage reader.

**3. The Tauri shim, moved from `tests/` into the shared package and made the default.** Lift
`installTauriShim` out of docs' `tests/disk.ts:37-133` unchanged, parameterise the backend module
specifier, and expose it both as a Playwright init script (what docs does now) and as a dev entry
point so `pnpm dev` in a browser gets real Tauri events too. Then `onAppEvent` (mail
`App.tsx:69-77`) is unnecessary and the app runs one code path in both environments. This is the
highest-value item in the audit: 97 lines that took real care to get right, written to a contract
rather than to convenience, and two of the three apps are the poorer for not having it. With it,
`emitEvent(page, name, payload)` and the delivery-count return become shared, and mail's
`narrateFirstSync` style of timed multi-step event script becomes a shared `script([...])`.

**4. Fixture loading.** Two patterns, both generalisable: docs' `import.meta.glob` corpus loader
(`corpus/load.ts`) as `loadCorpus(glob)`, and mail's `corpus!` macro (`fixtures.rs:10-19`) as a
shared Rust macro crate. Both encode the rule that adding a file puts it inside every sweep. Also
shared: `mutableCopy(fixture)` for the clone-at-module-load idiom in all three mocks, and
`undoLedger()` for mail's `undoable`/`runUndo` (`mockIpc.ts:292-314`), which any app with an undo
toast will want.

**5. Playwright helpers.** `settle`, `box`, `clockAt`, `MIDDAY`, `openDialog`, `token`,
`failCommands`, plus `makeOpenApp({ theme: "marginmail-theme", pane: "marginmail-pane" })` returning
an `openApp`, since only the seed keys differ between the two that have one.

**6. Assertions on tokens and icons.**

```ts
expectNoColourLiterals(root, ["src/ui", "src/screens"]);   // mail kit.spec.ts:82-96
expectThemesAgree(sheets, bootScript);                     // docs theme.test.ts
expectTokens(page, { "--list-w": "420px" });               // mail kit.spec.ts:62-67
expectContrast(page, selector, { min: 4.5 });              // cal legibility.spec.ts
expectAccessibleNames(page, selector);                     // cal legibility.spec.ts
expectIconsInSet(root, dirs);                              // does not exist yet
```

The first two are static scans over source and need no browser, so they can run under vitest in an
app with no Playwright, which is how margin gets its first test.

## What stays per app

Every measurement helper: calendar's `gridFit`, `axis`, `blocks`, `hourY`, `drag`; mail's `rows`,
`groups`, `paneMessages`, `paletteRows`, `actionBar`, `toast`; docs' `putCaret`, `caretIsIn`,
`watchDirty`. They read app-specific class names and encode app-specific timing, so sharing them
would be indirection over one caller each.

The fixtures themselves, and every spec file. A calendar of 12,067 events, a folder of markdown and
a mailbox of threads have nothing in common but the loading mechanism.

The Rust test convention. Mail's `#[cfg(test)] mod tests` beside the code and docs' integration
binaries are both right for their shape, and forcing one on the other buys nothing. The shareable
parts there are narrower: the `corpus!` macro and a `TempRepo` builder along the lines of
`support/notes_repo.rs`.
