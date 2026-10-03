# Migration

The order the work happens in, and what proves each step. Every phase leaves all four apps building
and shippable; nothing here is a long-lived branch.

## Preconditions

None of this starts until these are true. They are not precautions, they are the things that make
repo surgery safe.

**Commit and push the two dirty trees.** Margin Docs has 123 uncommitted files. Margin Mail has 123
uncommitted files on top of a single scaffold commit. Moving files between repos and rewriting
manifests across that is how work gets lost.

**Give Margin Mail a git remote.** It has none. That blocks the `uses:` reference for a reusable
workflow, blocks the sibling checkout other apps need, and blocks the shared repo's CI from building
it. Everything in [release.md](release.md) waits on this one command.

**Get Margin Docs' CI green.** Six of its last seven runs failed in `pnpm install --frozen-lockfile`
with `ENOENT: no such file or directory, scandir '/Users/runner/work/python/margin/shared'`, because
its workflow does a single checkout and its manifest points at a sibling repository. Margin Mail
already solved this with a second checkout and it was never carried back. Do that, as a stopgap, so
main is green before anything moves. Phase 1 deletes the stopgap.

## Fix before, or fix along the way

The audits turned up 30 defects. Most ride along with the extraction that touches them, but some have
to be settled first, because extracting a module means choosing which version wins and a bug you have
not decided about gets chosen by accident.

Fix first, because they block a phase or because they are shipping wrong today:

| Defect | Where | Why first |
|---|---|---|
| Margin Mail cannot compile for mobile | `lib.rs:203` has `mobile_entry_point` on `attach_account` rather than `run()` at `:250`; the three helpers called at `:307`, `:310`, `:314` are defined nowhere | The Rust extraction touches `lib.rs` in all four apps. Fixing it afterwards means fixing it twice. All three helpers exist in Margin Calendar and were meant to be ported |
| Margin's Google refresh token is in plaintext | `gdrive.rs:71`, `:153-157`, in `backup.json` | Same OAuth client and same grant as the two apps that seal theirs, so it sets the suite's real security level. `margin-secrets` should land on a repo that has already stopped doing this |
| Margin's PDF exports every heading at weight 400 | `pdf.rs:9-20`, `:57-62` load variable fonts, which Typst lays out at the default instance | The extraction has to take Margin Docs' static cuts. Decide that before the crate exists, not during |
| Margin's Typst escaper is `JSON.stringify` | `src/export/typst.ts:36-38` | Typst copies an unrecognised escape to the page verbatim, so `\b`, `\f` and braceless `\uXXXX` reach the PDF as visible backslashes. The busiest call site is every inline code span |
| Two apps ship placeholder updater pubkeys | Margin Docs and Margin Mail | Neither can ship a verifiable direct-download update. The shared release pipeline should not be built around a config that has never worked |
| No workflow sets `max-parallel: 1` | `margin:91`, `margin-caledar:85`, `margin-mail:85` set only `fail-fast: false` | tauri-action merges `latest.json` read-modify-write across platforms. The constraint was written down once and lost, which is the failure this whole exercise is about |
| Margin Calendar has no rate limit handling | A 429 becomes `ApiError::Other` at `google/api.rs:191-197`; the outbox counts it as a real attempt and five retire the write permanently at `push.rs:29`, `:375-381` | A user's write is silently dropped. `margin-http` fixes it, but the data loss is live now |

Everything else rides along and is listed in the document that owns it: the missing transactions and
the settings struct with 25 fields and one `serde(default)` in [rust-crates.md](rust-crates.md); the
theme key declared twice, the updater with no package-manager guard, and the keyboard normalisation
that makes `cmd+F` and `cmd+f` collide in [hooks.md](hooks.md); the unclamped row menu and the menu
with no escape layer in [ui-kit.md](ui-kit.md); the two justfile bugs and the unchecked TypeScript
projects in [toolchain.md](toolchain.md).

One of them is a fix rather than a saving and should be called out: three apps ship modals with
`role="dialog"` and `aria-modal="true"` and no focus trap behind them. Only Margin has
`useFocusTrap`. That arrives with `Sheet`.

## Phase 1: the shared repo exists and the build is not broken

Create `margin-shared` as its own repository, MIT licensed. Move `margin/shared` into it intact:
tokens, fonts, icons, the font binaries and `sync-fonts`. Tag `v0.1.0`. Repoint Margin, Margin Docs
and Margin Mail off the relative path, and add the dependency to Margin Calendar, which has never had
it.

Nothing else changes in this phase. No new tokens, no new components, no reconciliation. The point is
to move the existing thing to a place where a fresh clone works, and to prove the consumption
mechanism before anything depends on it.

**Proves it worked:** clone each of the four repos into an empty directory, `pnpm install`, `pnpm
build`. All four succeed. Today two of them cannot. Then Margin Docs' CI goes green with the stopgap
second checkout removed.

This phase also forces the licence decision. Margin is FSL-1.1-MIT, Margin Mail will be, Margin
Calendar and Margin Docs are MIT. The shared repo is MIT so all four can consume it. Make that
deliberately rather than discovering it mid-extraction.

## Phase 2: the design system

`@margin/tokens` and `@margin/fonts`, per [design-system.md](design-system.md).

Margin Calendar is 49 of 52 token values identical to the shared set already, so joining costs two
imports and a script pair. Promote the corrections two apps found independently, starting with
`--ink-faint`, which Calendar and Mail both moved to `#6e675b` light and `#8e8677` dark for the same
contrast reason while Margin and Docs kept the failing `#9b9484`. Promote the 19 tokens that exist in
two or three apps and not in shared. Collapse the base layer, which is already one file in four
byte-identical copies.

**Proves it worked:** a screenshot of the same screen in each app before and after, and a lint over
each app's CSS for colour literals outside the token layer. Margin has 15 hex and 17 `rgba()`
literals today; Calendar has one; Docs and Mail have none. The lint is what stops the drift
resuming, so it lands in this phase rather than later.

## Phase 3: the toolchain

`@margin/config`, per [toolchain.md](toolchain.md). The base tsconfig three apps already share byte
for byte, the Vite factory the four configs differ from only by port, the shared justfile recipes, and
the prose checker that currently exists in one repo.

Two things gate later phases and belong here. `@margin/ui` cannot ship source-only TSX unless each
app's tsconfig and Vite config compile TSX out of `node_modules`, and none does today. And the
checking gate has two holes: nothing type checks the Playwright specs, and nothing builds
`tsconfig.node.json`, so `vite.config.ts` is unchecked in all four and fails in two.

**Proves it worked:** one recipe name in every app runs the whole gate, and it is red in the places
the audits say it should be red today.

## Phase 4: the UI kit

`@margin/ui`, in the ranked order in [ui-kit.md](ui-kit.md): sheets and confirmation first, then list
navigation, the export preview, the primitives, and so on down.

The standing note in `shared/src/icons.ts:11-13` that says the `Icon` component is deliberately not
shared has to be reopened in the file, with the new reasoning, rather than quietly contradicted.

Do not do this before Phase 2. A component kit on four different token sets is a component kit that
looks different in four apps.

**Proves it worked:** Margin Mail's `Kit.tsx` renders every primitive in every state in both palettes,
and it keeps working after each promotion. That page is the regression test and it already exists.

## Phase 5: hooks and IPC

`@margin/hooks` and `@margin/ipc`, per [hooks.md](hooks.md). Start with the six things that are byte
identical today and verified by hash, because they drop in with no behaviour change. Order by
confidence, not by line count.

The winners are not all in one app, so no single repo is the source: Docs wins the theme and the
updater, Mail wins the keyboard registry and the toast and the `call` wrapper, Margin wins the focus
trap and relative time, Calendar wins clamp and contributes the packaged-by guard.

## Phase 6: the test harness

`@margin/test`, per [testing.md](testing.md). The three Playwright configs are one file with the port
swapped. The invoke stub is the thing worth sharing and three apps solved events three different ways,
with the correct one currently living in `tests/` where it cannot be reused.

This is the highest-value item in the whole consolidation for day-to-day work, because it is what
lets a change be verified without touching the running dev server, which
[guidelines/working-together.md](guidelines/working-together.md) forbids.

Margin comes last here and is new work rather than migration: it has no tests of any kind and no
`src/ipc.ts` seam to plug a fixture into.

## Phase 7: the Rust crates

Per [rust-crates.md](rust-crates.md), then [accounts.md](accounts.md), then
[typesetting.md](typesetting.md). Consumed as cargo git dependencies pinned to a tag, because a path
dependency across checkouts fails on a fresh clone in exactly the way the npm one already does.

Order within the phase: `margin-log` first and alone, because it is about 100 lines, it is the only
logging in the suite, and every later phase debugs better with it. Then the Tauri shell, where 73
distinct lines of `lib.rs` are verbatim identical in all four files. Then SQLite, done for the two
defects it fixes rather than the volume. Then secrets, Google and HTTP together, since 1,552 of
Margin Calendar's 1,675 non-test OAuth lines exist verbatim in Margin Mail. Then typesetting,
grammar and the macOS integrations.

Close the version drift in the same pass: rusqlite 0.37 against 0.40, reqwest 0.12 against 0.13,
chacha20poly1305 0.10 against 0.11, fontdb 0.23 against 0.24.

Note what is deliberately not built: no shared settings crate, no shared error crate, no shared
filesystem crate, no shared async crate, and no shared sync engine. Each refusal has evidence behind
it in its own document, and the sync engine refusal is the most important one in the plan.

## Phase 8: release

Per [release.md](release.md). The `prepare` job is 81 lines in two apps and differs by one line. No
repo has the good version of the pipeline: every good idea lives in exactly one repo and one app has
no CI workflow at all.

This is late deliberately. A reusable workflow is only worth building once the four repos agree about
what a build is, which is what phases 1 through 7 settle.

**Proves it worked:** each app's own workflow file is under 20 lines, and a release of each app
produces a signed, notarised, verifiable artifact.

## Phase 9: names

Per [naming.md](naming.md). Last, because a rename during extraction is a rename of a moving target,
and because two of the renames are free only once nothing points at the old paths.

The bundle identifiers stay frozen. Changing one orphans the app data directory, the sealed-secret
service name, the OAuth redirect scheme registered with Google, the macOS notification settings deep
link, the App Store record and the Homebrew cask, and breaks the update path for every existing
install.

## What "done" looks like

A fresh clone of any of the five repos installs and builds with no sibling checkout. The same button
exists once. A token changed in one place changes in four apps. An icon is aligned by two CSS rules
rather than by a nudge at each call site. A failure is written to a log file in every app rather than
one. A change can be verified in a browser against fixtures without touching the running dev server.
And the rules the four apps are built by are in `guidelines/` in a repository, rather than in memory
files on one machine.
