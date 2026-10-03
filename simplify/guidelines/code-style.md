# Code style

## Formatting

No prettier. No formatter of any kind. The source is hand-formatted at roughly 120 columns and there
is no config to make a formatter agree with it.

Running `npx prettier --write` rewrites a file at prettier's 80-column default and turns a 60-line
change into a 340-line diff, which destroys reviewability and churns files the change never touched.
If one has already run, `git checkout <file>` and redo the edits by hand.

Match the surrounding indentation. Make surgical edits.

There is no eslint either. The gate is `tsc`, `cargo check` and the test suites.

## Comments

The rule as originally written was "no code comments: make the code readable instead". That is still
right about one kind of comment and wrong about another, and the codebase has moved.

A comment never says what the code does. If you are about to write one, rename something or extract
a function instead.

A comment does say why a decision was made, when the reason is not recoverable from the code. The
best examples in the suite are `shared/src/fonts.ts`, `shared/src/icons.ts` and the dependency
blocks in Margin Mail's and Margin Docs' `Cargo.toml`, which explain why a version is pinned exactly,
why `keyring` is not used, why `bundled` is the feature that gets FTS5. Every one of those is a
decision somebody would otherwise undo by accident.

The test: delete the comment and ask whether a competent person would make the same mistake twice.
If yes, keep it. If it just narrates the line below, cut it.

## Tests

The old rule was "never commit tests, verify by running the real product". That rule is dead. It was
true of Margin alone and Margin has no test script to this day, but Margin Calendar, Margin Docs and
Margin Mail all ship vitest unit tests and Playwright suites, and they catch things.

The position now: unit tests go beside the module they test as `Name.test.ts`, and they test pure
model code (`GridModel`, `AgendaModel`, `QuickCreateModel`, `theme`, `width`, `providers`), never
rendering. Playwright suites drive the app in a browser against the `src/dev` fixtures.

Running the real product is still required, and still the last step. Tests are in addition to it,
not instead of it. See [working-together.md](working-together.md).

If the user wants the old rule back for a given app, they will say so. Do not delete an existing
suite on the strength of a memory written before it existed.

## Cross-platform over per-OS native

Before adding a dependency with per-OS backends, check it covers all five targets: macOS, Linux,
Windows, Android, iOS. If it does not, prefer one implementation that works everywhere, and state
the security or capability trade plainly in the code rather than hiding it behind a fallback chain.

`keyring` is the worked example and the reason for the rule. It had four ways of reaching one real
implementation. macOS was already excluded because the Keychain ties an item to the code signature
and re-prompts on every rebuild; Android has no backend at all; on Linux the Secret Service is
missing on exactly the minimal window managers that most wanted it. It was replaced everywhere by
XChaCha20-Poly1305 sealed files in the app data directory. Accepting a weaker but uniform mechanism
is usually the right answer here.
