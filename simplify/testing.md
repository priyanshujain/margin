# Testing: the dev harness and `@margin/test`

Scope: the browser fixture harness, the Playwright configs and suites, the spec helpers, the Rust
fixture corpora, and what CI runs. The production half of `@margin/ipc` (the `call` wrapper, the
phase union, `listen`) belongs to [hooks.md](hooks.md); this document owns the dev-mode half of the
same package. Short names: `margin` = `python/margin`, `calendar` = `python/margin-caledar`,
`docs` = `rust/margin-editor`, `mail` = `rust/margin-mail`, all under
`/Users/pj/Workspace/projects`. Line numbers and counts are as of 2026-09-06.

## The harness is the most valuable thing in this consolidation

[guidelines/working-together.md](guidelines/working-together.md) forbids starting a dev server: the
user keeps one running, Vite is on `strictPort`, and in Margin Mail a dev instance shares the real
app data directory, so a second one is a live-data accident. That leaves one way for an agent to see
whether a change works without asking the user to look: a scratch Vite on a spare port with the
invoke boundary stubbed, driven by Playwright. The stub is each app's `src/dev`, and it is worth
more than any component in `@margin/ui`, because without it every verification ends in "try it
yourself", which the same guideline also forbids.

What each app has today:

- calendar: a 356 line `src/dev`, a 12 command mock, no faked events at all, and a suite happy to
  pass against any checkout's dev server.
- docs: the best harness in the suite, a hand-rolled Tauri shim so the real `@tauri-apps/api` event
  plugin runs in a plain tab, plus the only identity check, both locked inside `tests/` where
  `pnpm dev` cannot reach them.
- mail: the largest fixture at 3,341 lines and 105 commands, events faked as window `CustomEvent`s
  with an app-side bridge, and the only sane screenshot policy.
- margin: nothing. No `tests/`, no `src/dev`, no vitest, no `test` script (`package.json:6-13`), and
  `src/ipc.ts` is 43 lines calling `invoke` directly behind an `isDesktop` early return, so there is
  no seam a fixture could plug into.

Margin is not a migration. It builds the seam the other three already have, which is why it is last
in the sequence rather than first.

## The state of play

| | margin | calendar | docs | mail |
| --- | --- | --- | --- | --- |
| `test` / `test:ui` scripts | neither | both | both | both |
| `vitest` / `@playwright/test` | neither | 3.2.4 / 1.62.1 | 3.2.4 / 1.62.1 | 3.2.4 / 1.62.1 |
| `src/dev` lines | absent | 356 | 1,040 | 3,341 |
| mock `case` labels | none | 12 | 35 | 105 |
| spec files / `test(` blocks | 0 / 0 | 10 / 131 | 14 / 110 | 24 / 253 |
| spec lines, helpers excluded | 0 | 2,050 | 3,717 | 6,445 |
| colocated `*.test.ts` files / tests | 0 / 0 | 12 / 211 | 32 / 699 | 8 / 81 |
| Rust `#[test]` | 0 | 79 | 0, all integration | 531 |
| `src-tauri/tests/` | no | no | 8 binaries plus support | no |
| `src-tauri/fixtures/` | no | no | no | 76 files |

Margin has no tests of any kind, and its `vite.config.ts` imports from `vite` rather than
`vitest/config`, so there is not even a `test` block to add to. It is also the app with the most
design drift, 16 hex literals and 18 rgba values outside its token layer against zero in docs and
mail ([design-system.md](design-system.md)); the two facts are related. Docs' six `_audit*.spec.ts`
files have since been deleted, so its count is 14 specs, not the 19 the audit found.

## `@margin/test`, the Playwright side

The three configs are one file with the port swapped. Byte identical across all three: `testDir`,
`fullyParallel`, `retries: 0` with the same comment about a test that only passes on the second go,
the list reporter, the cache `outputDir`, both timeouts, the 1440x900 viewport, `en-GB`, the trace
and screenshot policies, one chromium project with the identical comment on why it is not
`devices["Desktop Chrome"]`, and a `webServer` running `pnpm dev` with `reuseExistingServer: true`.
So the whole file becomes a call:

```ts
import { marginPlaywrightConfig } from "@margin/test/playwright";

export default marginPlaywrightConfig({
  port: 1450,
  witnesses: ["src/ipc.ts", "src/dev/backend.ts", "src/dev/fixture.ts"],
});

interface MarginPlaywrightOptions {
  /** 1430 calendar, 1440 docs, 1450 mail, 1460 margin. baseURL and webServer.url come off it. */
  port: number;
  /** Source files byte-compared against the served copy before any spec runs. Empty turns the check off. */
  witnesses: string[];
  /** Defaults to Asia/Kolkata. Pass null for a suite that must read the runner's own zone. */
  timezoneId?: string | null;
  use?: PlaywrightTestConfig["use"];   // merged over the defaults, for the rare real difference
}
```

Two genuine differences exist and both become defaults. `timezoneId: "Asia/Kolkata"` is set in
calendar (`playwright.config.ts:34`) and mail (`:31`), which anchor their fixtures to the browser's
local day, and absent in docs, which has no clock; the factory defaults it on and docs passes
`timezoneId: null`, because pinning the zone by accident is harmless and failing in another zone is
not. The second is `globalSetup` in docs only (`:14`), and it becomes the default by way of
`witnesses`.

The identity check is the part of docs' setup the other two most need. `tests/identity.ts` is 106
lines run once before any spec: for each of five witness files it fetches
`${baseURL}/${witness}?raw`, decodes the string literal Vite serves by hand rather than by
evaluating it (`:38-80`, since running what an untrusted server sends to decide whether to trust it
has the order wrong), and compares it byte for byte against disk. Its header (`:1-15`) says why:
`reuseExistingServer` on a fixed port is what makes the suite quick and also what lets it talk to a
server another checkout left behind, where a pass proves nothing and a failure sends you hunting
through a file you never touched.

Calendar and mail have no such check, so both suites are currently happy to pass against anybody's
copy. With four apps on four adjacent ports and one machine, that is not hypothetical.

The factory supplies `globalSetup`, so no app wires it, and `witnesses` is the only per-app input:
for mail `src/ipc.ts`, the dev backend and the fixture; for docs the schema, the serializer and the
save path it already names at `identity.ts:25-31`. Saved: roughly 45 lines per app, and "all three
run at 1440x900 with no retries" becomes a fact rather than a coincidence.

## `@margin/ipc`, the dev harness side

All three apps switch dev mode on identically in `src/ipc.ts` (calendar `:192-197`, docs `:288-292`,
mail `:748-752`): in a `DEV` build with no `__TAURI_INTERNALS__`, `call` imports `./dev/mockIpc` and
routes there, otherwise it invokes. Mail's differs in one way that matters and wins: its `catch` arm
posts the failure to `log_note` before rethrowing, skipping `log_note` itself so it cannot loop
(`ipc.ts:752-759`). That is [guidelines/errors-and-feedback.md](guidelines/errors-and-feedback.md)
in code, and the shared `makeCall(loadBackend)` carries it, alongside `devFlag(key)`, the eight-line
localStorage reader that exists three times today.

### The typed command registry

`mockCall` is a `switch (command)` in all three, returning `as unknown as T` at every arm and
throwing on an unknown command (calendar `mockIpc.ts:131-132`). There is no type link between a
command name, its arguments and its return: calendar casts args to `Record<string, never>` (`:41`)
and then per field. Replace it with a table:

```ts
// src/dev/backend.ts
import { defineBackend, devFlag } from "@margin/ipc/dev";

export const backend = defineBackend({
  accounts_list: () => (devFlag("marginmail-dev-empty") ? [] : accounts),
  thread_view: ({ key }: { key: string }): ThreadView => viewOf(byKey(key)),
});
export type Command = keyof typeof backend;
export const { mockCall, has, commands, emit, script } = backend;
```

The arg cast at every arm and the `as unknown as T` at every return both disappear, and a command
declared in `dto.rs` with no handler becomes checkable in one place instead of a runtime surprise
three screens later. `has` and `commands` are what makes that check writable as a vitest.

### Faking events, three ways

Calendar does not fake them: `App.tsx:52` is `if (!isTauri) return;` ahead of every `listen`, so
`menu-action`, `auth`, `sync-progress` and `store-changed` never arrive in a browser and everything
they drive is unreachable from the suite, including the `auth` listener its own comment calls the
only thing that ever learns a mobile sign-in worked.

Mail dispatches window `CustomEvent`s from inside the mock under the same names, with an optional
delay so a state that would otherwise last one frame is observable (`mockIpc.ts:237-248`;
`narrateFirstSync` at `:637` walks a five-step sync). The app side is `onAppEvent` (`App.tsx:69-77`),
seven lines picking `listen` or `window.addEventListener` off `isTauri`. Cheapest correct answer,
and it leaves the Tauri arm of that fork, the arm that ships, never exercised in a browser.

Docs does the right thing. `installTauriShim` (`tests/disk.ts:37-133`) installs a hand-rolled
`__TAURI_INTERNALS__` at document start: `invoke`, `transformCallback`, `unregisterCallback`,
`runCallback`, `plugin:event|listen` and `|unlisten`, a listener map, `metadata`, `convertFileSrc`
and `__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener`. Its comment (`:27-36`) records that it is
deliberately not `@tauri-apps/api/mocks`, which cannot be reached from an init script, and written
to the contract rather than to convenience. `emit` returns a delivery count per event (`:66-75`), so
a test can tell a working subscription from a payload that fell on the floor.

Docs is the one to keep: the app runs the real `@tauri-apps/api`, so the branch under test is the
branch that ships, and mail's `onAppEvent` stops being necessary. It is not reusable today for two
reasons, that it lives in `tests/` so a person running `pnpm dev` by hand gets no events, and that
the backend module specifier is hard-coded at `disk.ts:79`.

### What an app implements

```ts
// src/dev/harness.ts, imported by src/main.tsx behind import.meta.env.DEV and by the spec helper
import { installTauriShim } from "@margin/ipc/dev";
installTauriShim({ backend: "/src/dev/backend.ts" });
```

It must stay self-contained with no module-scope closure, because Playwright serialises it into the
page through `addInitScript`, which is why the specifier is an argument. From the spec side:

```ts
import { emitEvent, driveFixture } from "@margin/test/harness";
const delivered = await emitEvent(page, "store-changed", "threads");  // returns the listener count
await driveFixture(page, "external.rename", ["notes/a.md", "notes/b.md"]);
```

`driveFixture` generalises docs' `change`/`ask` pair (`disk.ts:135-144`): the app exports an
`external` surface of functions that mutate the fixture the way another program would and return the
events the backend would have emitted, and the harness puts them on the bus. Docs' `pauseWrites` and
`resumeWrites` (`mockIpc.ts:679-695`) stay app-specific; the mechanism does not. Mail's timed
narration becomes `script([{ after: 0, event, payload }, ...])`, and dev flags keep their per-app
names, `<app>-dev-<thing>` being uniform already.

## The spec helpers

Calendar and mail both have `tests/app.ts`, 370 and 401 lines, and 10 of 10 and 23 of 24 specs
import from it (the exception, mail's `kit.spec.ts`, is mostly static scans). These six are code
identical between the two and differ only in doc comments:

| helper | calendar | mail | lines |
| --- | --- | --- | --- |
| `clockAt` + `MIDDAY` | `app.ts:41-53` | `app.ts:46-58` | 13 |
| `openApp` | `:61-90` | `:66-90` | 25, seed keys only |
| `settle` | `:105-112` | `:99-106` | 8, third copy at docs `caret.ts:46-53` |
| `box` | `:114-124` | `:108-118` | 11 |
| `openDialog` | `:288-293` | `:188-193` | 6 |
| `contrastOf` | `:306-366` | `:219-278` | 60 |

`contrastOf` is the one that took real work: it composites every translucent background between the
element and the page through a 1x1 canvas, because `oklch()` and `color-mix()` otherwise read as
transparent. The two apps that have it are the two that independently found the `--ink-faint`
defect. Two more mail helpers are generic despite being written for mail: `token(page, name)`
(`app.ts:180-185`) and `failCommands(page, commands)` (`app.ts:343-360`), which answers the request
for the dev backend module with a shim forwarding to the real one (`?real`) and rejecting the named
commands. That is the only way to test a failure path when the backend is in the page and not on the
wire, and 18 lines that work unchanged in any of the four.

`openApp` becomes a factory, since only the seed keys differ:

```ts
export const openApp = makeOpenApp({ theme: "marginmail-theme", pane: "marginmail-pane" });
```

The `__test-seeded` sentinel goes with it: the seed is written once per context and not on every
navigation, so a test that reloads to check what survived is not silently reset underneath itself.

Docs has no shared spec helper at all. Its 14 specs define 16 local opener functions (`openReadme`,
`openHandbook`, `openWriting`, `openFolder`, `open`, `openPreview`) and all 14 inline the same
`margindocs-recents` seed. Adopting `makeOpenApp` is the single biggest deletion in this document.

## The Rust fixture story

Mail's `src-tauri/fixtures/` is 76 files: 36 `.eml` messages as they come off the wire (CRLF
throughout, half not UTF-8, ISO-8859-1 and ISO-2022-JP among them), 36 matching `golden/*.txt` and 4
`autoconfig/*.xml`. A `corpus!` macro (`fixtures.rs:10-19`) compiles them into the test binary as
one `include_bytes!` const per file plus an `all()` returning every pair, and `fixtures.rs:58-102`
is itself a test: every fixture has a header/body break, no bare LF, a plausible date, a parseable
From. Beside it, `provider/fake.rs` (612 lines, `#![cfg(test)]`) is an in-memory mailbox with
paging, a history log and scripted failures, and the only reason the sync engine is testable
without credentials.

There are two hand-maintained lists of that one directory and they have drifted. `fixtures.rs:21-52`
names 30 files; a second `corpus!` macro at `mime/parse.rs:627-672` names 36. The six missing from
the first are the `surface-*.eml` set (`dark-inline-text`, `newsletter-background-image`,
`newsletter-bgcolor`, `plain-html`, `sender-dark-design`, `wrapper-background`), so
`every_fixture_is_a_message` checks none of them. That is the exact failure docs wrote a paragraph
about at `src/markdown/corpus/load.ts:6-9`, where naming folders explicitly left twenty adversarial
files inside the corpus and outside every gate that read it. The fix there was
`import.meta.glob("./*/*.md")`; the fix here is the same idea.

The others have nothing comparable. Docs builds its Rust fixture at runtime:
`src-tauri/tests/support/notes_repo.rs` (339 lines) creates a real git repository per test binary,
copies 12 documents out of `src/markdown/corpus/real` so there is one corpus and not two, and
generates 13,000 files under a vendored `node_modules`, because several tests need a folder large
enough that skipping it beats walking it. Calendar has 79 `#[test]`s and no fixtures, margin
neither.

Worth sharing? Not yet, and this document should say so rather than pad the package list. The macro
is ten lines with one consumer, a crate for it would be indirection over a single call site, and
[risks.md](risks.md) is explicit that sharing is justified by measured duplication. What is worth
doing now is fixing the drift inside mail (glob the directory with `include_dir`, delete both lists)
and writing the rule into [guidelines/code-style.md](guidelines/code-style.md): a corpus is a
directory, never a list, because adding a file must put it inside every sweep. `TempRepo` moves into
a shared crate when a second app wants a git-backed temporary directory, and none does today.

## Two gaps that are not duplication

### Nothing asserts anything about icons

Design-system assertions exist in three shapes and none covers icons. Mail's `tests/kit.spec.ts`
(132 lines) renders `#/kit` in both palettes, asserts geometry read from custom properties
(`--list-w` is `420px`, a row 46px, an avatar 30x30), then runs three static `node:fs` scans: no hex
literal in any stylesheet under `src/ui` or `src/screens`, every `<input>` carrying `NO_AUTOFILL` or
an explicit `autoComplete`, no file saying "keychain". Calendar's `legibility.spec.ts` is runtime:
contrast, no block reading "Untitled", a non-empty accessible name on every block, both themes.
Docs' is a vitest: `src/theme.test.ts` reads three stylesheets plus `index.html`'s pre-bundle boot
script and asserts all four declare the same variable set, because a missing dark variable falls
back silently to the warm light value in `:root`. A grep for icon assertions across all three suites
returns nothing, while [risks.md](risks.md) records five independent fixes for the same icon
baseline problem and eleven implementations of a square icon button. What `@margin/test` ships:

```ts
expectThemesAgree(sheets, bootScript);                   // docs src/theme.test.ts, vitest, no browser
expectNoColourLiterals(root, ["src/ui", "src/screens"]); // mail kit.spec.ts:80-96, vitest, no browser
expectIconsFromSet(root, dirs);                          // no path data outside @margin/icons, vitest
expectIconGrid(icons);                                   // every path inside the 24 unit box, vitest
expectTokens(page, { "--list-w": "420px" });             // mail kit.spec.ts:58-79
expectContrast(page, selector, { min: 4.5 });            // calendar legibility.spec.ts, built on contrastOf
expectAccessibleNames(page, selector);                   // calendar legibility.spec.ts
expectIconOptical(page, ".icon-button");                 // painted glyph box centred within 1px
```

The first four are static scans over source and need no browser, so they run under vitest. That is
what makes them the first tests Margin ever gets: they land before it has a Playwright config, a
fixture or a dev harness, and they fail today on the drift [design-system.md](design-system.md)
measured.

### Known failures are recorded nowhere in any tree

The only markers anywhere are `guide-shots.spec.ts:19-22`, an intentional `GUIDE_SHOTS` env gate,
and a `test.fail` at `external-changes.spec.ts:320` kept so it turns red the day the defect is
fixed. The four Margin Mail browser failures are written down only in
[guidelines/app-facts.md](guidelines/app-facts.md), which is in this plan directory and not in the
repo that fails.

They are not one thing and should not be recorded as one. Three (`shell.spec.ts` "New for you above
Previously seen", `snooze.spec.ts` "Back above New for you", `triage.spec.ts` "Mark all as seen is a
link on the heading") assert Inbox group heads the app deliberately no longer draws: stale tests,
and the work is rewriting them to the current design. The fourth, `kit.spec.ts` "every text field
tells the webview not to fill it in", flags a real input in `Kit.tsx` and one in `Settings.tsx`.
That is the test doing its job, and recording it as a known failure would be weakening a test to
reach green, which `margin-editor/docs/conventions.md:74` forbids in as many words. Fix it.

The record goes in `tests/known-failures.md` per app: one heading per failure naming the spec file
and the test title (matched on the name, never the line number), what it asserts, why it fails and
which kind it is. `@margin/test` ships the checker that reads it, so a suite whose failures match
the file exactly exits with a distinct code and one that fails anything else does not. Without the
checker the file rots into a list of excuses within a month.

## CI

Playwright runs in no app's CI. Calendar's `ci.yml:31` and `:70` run `pnpm test` and `cargo test`;
docs' `:34`, `:58` and `:78` the same plus a separate `--test-threads=1` binary; mail's `:49` and
`:92` the same plus `pnpm fonts:check` and `node scripts/docs-check.mjs`. The three justfiles agree
line for line that `just test-ui` is `pnpm test:ui`, and nothing in CI calls it. What should run
where:

- Every push, every app: `pnpm build`, `pnpm test`, `cargo test` and the new static assertions.
  Margin joins this list first and gets `pnpm test` for the first time.
- Every push, the three with a suite and then all four: `pnpm test:ui`, after
  `pnpm exec playwright install --with-deps chromium`. Two minutes on a runner already building the
  Rust side, and the only check that the harness itself still works. The identity check is a no-op
  there, where Playwright starts its own server, which is fine: it costs five fetches.
- Never in CI: `guide-shots.spec.ts`, which writes committed files, and mail's eight `#[ignore]`
  Rust tests, six of them "hits the network" in `imap/discover.rs`.
- The shared repo's own CI, per [risks.md](risks.md): check out all four apps against the candidate
  tag and run each one's `pnpm test` and `pnpm test:ui`. That is the only place the four are checked
  together, and what turns a token rename from a silent break in the app nobody was working on into
  a red build before the tag exists.

Worth wiring while in here: nothing runs `tsc -p tests/tsconfig.json` anywhere, so every spec file
is type checked by an editor and nothing else. That file is the same nine options in all three, it
moves into `@margin/config`, and the gate gains a line.

## Per app, exactly what changes

**1. Docs, because it owns the two pieces worth extracting.** Move `installTauriShim` out of
`tests/disk.ts` into `@margin/ipc/dev`, unchanged except for the backend specifier becoming an
argument, and call it from a dev entry point as well as from `addInitScript`, so `pnpm dev` in a
browser gets real Tauri events for the first time. Move `identity.ts` into `@margin/test` behind
`witnesses`. Convert `mockIpc.ts` to `defineBackend`, keeping `external`. Adopt `makeOpenApp` across
all 14 specs and delete the 16 openers. Move `theme.test.ts` to `expectThemesAgree`. Check: the
suite green, in particular `external-changes.spec.ts`, which drives the watcher over the shim, and
its `test.fail` at `:320` still failing.

**2. Mail, the biggest fixture and the most to gain.** Take the shim, delete `onAppEvent`
(`App.tsx:69-77`) and the `CustomEvent` arm of `emit` (`mockIpc.ts:237-248`), so the app runs one
code path in both environments and the `listen` branch that ships is the branch under test. Convert
105 `case` labels to `defineBackend`, the largest single piece of work here and the one to do in
slices by screen. Contribute `token` and `failCommands`, adopt the shared helpers, keep the
measurement ones. Turn `kit.spec.ts`'s scans into `expectNoColourLiterals` and friends, and fix the
two inputs it flags rather than recording them. Rewrite the three stale group-head assertions. Fix
the corpus drift. Check: 24 specs green, `cargo test` at 531 green, `just install` last.

**3. Calendar, which gains events it has never had.** Take the shim and delete `App.tsx:52`'s
`if (!isTauri) return;`, then write the first specs that drive `menu-action`, `auth` and
`sync-progress`, none of which has ever been reachable from a browser. Adopt the shared helpers,
contribute `contrastOf` and `legibility.spec.ts`'s runtime assertions upward, add `witnesses`. Check
`legibility.spec.ts` and `touch.spec.ts`, the second because its file-scope `test.use` is the one
thing the factory must not disturb.

**4. Margin, which is new work and not a migration.** In this order, each step useful on its own:

1. `vitest`, a `test` script, a `test` block in `vite.config.ts`, and `expectNoColourLiterals` plus
   `expectThemesAgree` pointed at its stylesheets. They fail on day one against the 16 hex and 18
   rgba literals in `app.css`, which is the point: the first test catches the drift that motivated
   the plan.
2. Split `src/ipc.ts` into a `call` seam using `makeCall`, so a fixture has somewhere to plug in.
   Today the 43 lines call `invoke` directly and each function early-returns on `!isDesktop`, so a
   browser gets silence rather than data.
3. `src/dev/fixture.ts` and `src/dev/backend.ts`: a library of two or three books with images, which
   is what its data model is, plus the writing-tools and PDF commands stubbed.
4. `installTauriShim` at the dev entry point, `@playwright/test`,
   `marginPlaywrightConfig({ port: 1460 })`, and a smoke spec that opens a book, types and asserts
   the word count. Then the rest: the export path, the proofing colours, the settings panel.

Nothing in steps 2 to 4 blocks the shared token migration, and step 1 should land before it, because
[design-system.md](design-system.md) notes Margin is the app whose base sheet extraction has no
suite to catch a mistake.

## What stays per app

Every measurement helper. Calendar's `gridFit`, `axis`, `blocks`, `headerDates`, `hourY`, `columnX`
and `drag`; mail's `rows`, `groups`, `paneMessages`, `paletteRows`, `actionBar`, `checkedRows`,
`bodyText`, `toast`, `listScroll`, `place` and `openRow`; docs' `putCaret` and `caretIsIn`, whose
28-line header documents why nothing in the suite presses End to move a caret, and `watchDirty`, a
MutationObserver installed before typing so the 500ms autosave cannot be raced. They read
app-specific class names and encode app-specific timing, so sharing them would be indirection over
one caller each.

The fixtures and every spec file: a calendar of 12,067 events, a folder of markdown and a mailbox of
threads have nothing in common but the loading mechanism. Mail's `#[cfg(test)] mod tests` beside the code, with fifteen modules large enough for a sibling
`<module>/tests.rs`, and docs' eight integration binaries are both right for their shape, and
forcing either on the other buys nothing.

The screenshot policy is the one convention worth copying by hand rather than packaging. Mail writes
28 screenshots across 16 specs into a gitignored `screenshots/` (`.gitignore:41`), so an ordinary
run leaves the tree clean, while the 10 pictures that ship inside the bundle go to a committed
`public/guide/` behind `test.skip(() => !process.env.GUIDE_SHOTS)` and `just guide-shots`. Only
mail has that two-tier rule, it is right, and it is four lines rather than a package.
