# Repo layout

Where shared code lives, how four repositories consume it, and what each package is called. Every
other document in this directory uses the names defined here.

## The problem with what exists

`margin-shared` is a real package with the right contents, living in the wrong place. It sits inside
Margin's git repository at `python/margin/shared`, and Margin Docs and Margin Mail depend on it as
`"margin-shared": "file:../../python/margin/shared"`, a path that walks out of one repository and
into a sibling checkout.

On this machine pnpm has resolved that to a symlink and it works. Nowhere else does it. A fresh
clone of Margin Docs fails `pnpm install` unless Margin happens to be checked out at exactly that
relative path. No CI runner reproduces it. No second contributor reproduces it. The user's own
machine reproduces it only because of the order things were created in.

Margin Calendar sidesteps the whole thing by not depending on it at all and keeping its own copies of
the same tokens, which is how the tokens drifted.

There is a second problem underneath. Margin is FSL-1.1-MIT and Margin Mail will be. Margin Calendar
and Margin Docs are MIT. Shared code cannot be under two licences, and shared code that lives inside
the FSL repo inherits the wrong one.

## The shape of the answer

A fifth repository, `margin-shared`, holding npm packages under `packages/` and Rust crates under
`crates/`, licensed MIT so all four apps can consume it whatever their own licence says.

The four app repositories stay separate, as the user wants. They depend on the shared repo by
version, not by relative path.

A monorepo containing all four apps would be simpler than this. Atomic changes across an app and its
dependency, one lockfile, one CI, no publishing step, no version skew. It is the obvious engineering
answer and it should be said out loud rather than implied. It is not what is being built here,
because the four apps ship separately, have separate release cadences, have separate App Store
records, and one of them is FSL while others are MIT. Those are real reasons and the decision stands,
but the cost is real too: every shared change becomes a two-step, and [risks.md](risks.md) says what
that costs in practice.

## The npm packages

Scope `@margin`. Every one of them is source-only with no build step, resolved through Vite the way
`margin-shared` already is, because a build step in a design token package buys nothing and costs a
watch mode.

`@margin/tokens`. The CSS custom properties, the base reset, the focus ring, the scrollbar, the
title bar metrics, light and dark under `data-theme`. One stylesheet, imported first by every app.
This is the single highest-value package and the one to do first.

`@margin/fonts`. The six bundled faces: the catalogue in TypeScript, the `@font-face` block, the
variable font binaries and their licences, and the `sync-fonts` binary that vendors them into an
app's `public/fonts` for the Rust PDF exporters to read with `include_bytes!`. This is today's
`shared/src/fonts.ts` and `shared/fonts/`, moved intact.

`@margin/icons`. The glyph paths on a 24 unit grid for a 1.6 stroke, as bare strings with no React
dependency. Today's `shared/src/icons.ts` plus every path the four apps have each drawn separately.

`@margin/ui`. The React primitives. This is the package that does not exist today and is the reason
the same button gets built four times: `Icon`, `Button`, `IconButton`, `Field`, `Switch`, `Select`,
`Dialog`, `Confirm`, `Sheet`, `Toast`, `Banner`, `Menu`, `RowMenu`, `ResizeHandle`, `FindBar`,
`Palette`, `SettingsShell`, `EmptyState`, `Spinner`, `Kbd`. Every one of them carries the busy and
disabled states that [guidelines/errors-and-feedback.md](guidelines/errors-and-feedback.md) requires,
so no app can forget them. Details in [ui-kit.md](ui-kit.md).

`@margin/hooks`. `useMedia` and `useCompact`, the theme hook and its `data-theme` writer, the escape
stack, the keyboard registry and its scope stack, focus trapping and restoration, and the small
utilities each app reinvented: clamp, debounce, relative time, byte size, platform detection.

`@margin/ipc`. The typed wrapper around Tauri's `invoke` and `listen`: the `call` function that logs
every failure, the phase union type the conventions require, the error shape that crosses the
boundary, and the `isTauri` check. Also the dev-mode stub that lets the app run in a plain browser
against fixtures, which is currently reinvented in three `src/dev` directories.

`@margin/test`. The Playwright config factory, the fixture loader, and the assertions every app
should share: that a control has a busy state, that a colour resolves to a token, that an icon is
aligned. Details in [testing.md](testing.md).

`@margin/config`. A base `tsconfig.json` to extend and a Vite config factory taking the app's name
and port. Details in [toolchain.md](toolchain.md).

`@margin/typeset`. The TypeScript half of the Typst pipeline, which is the half that matters for
correctness: the string escaper and its sanitiser. Both apps build Typst source in TypeScript, so a
Rust-only escaper would not fix the bug that put JSON escape sequences on the page. The two halves
share one table of test vectors. Details in [typesetting.md](typesetting.md).

## The Rust crates

Same repository, under `crates/`, consumed as cargo git dependencies pinned to a tag.

This list was drafted before the audits ran and the audits disagree with parts of it.
[rust-crates.md](rust-crates.md) is the authority: it argues against a shared settings crate, a
shared error crate, a shared filesystem crate and a shared async crate, with the evidence, and it
renames others. Where the two documents differ, that one is right and says why.

`margin-paths`. App data directory, library directory, atomic write, trash, path validation. Four
apps have four versions of this and Margin's already lives in `library.rs` as `pub(crate)` helpers.

`margin-log`. The log file in the app data directory, with rotation. Margin Mail has it; the other
three log nothing, which is why an error report from them starts with guesswork.

`margin-db`. Opening a rusqlite connection with the busy timeout and WAL that three apps each set
separately, the migration runner and its version table, and the FTS5 helpers.

`margin-secrets`. The XChaCha20-Poly1305 sealed file that replaced `keyring`, with the key derivation
and the atomic replace. Margin Calendar and Margin Mail have near-identical copies at different
crate versions.

`margin-google`. The OAuth client: PKCE, the loopback listener on desktop, the deep link scheme on
mobile, token exchange, refresh with clock skew, revocation, and the multi-account store. Three apps
talk to Google and two have full implementations of this.

`margin-http`. One `reqwest` client with the timeouts, retry and backoff policy, and the user agent,
so a rate limit is handled the same way everywhere.

`margin-ipc`. The serde conventions for types crossing into the webview and the error type that
crosses with them.

`margin-typeset`. The Typst pipeline: preamble generation, the font resolver, escaping user text into
Typst source, image handling, page setup, error mapping. Plus the `fontdb` catalogue. Margin and
Margin Docs each carry a copy of all of it. Details in [typesetting.md](typesetting.md).

`margin-mac`. The macOS integrations behind one `cfg(target_os = "macos")` boundary: `NSSpellChecker`,
Apple Writing Tools, `UNUserNotificationCenter` posting, and the title bar inset work. Margin and
Margin Docs duplicate the first two. Only Margin Mail posts notifications at all, and it is the only
one that does so correctly; the other three do not have the bug because they do not have the feature,
which is the state [typesetting.md](typesetting.md) records.

`margin-grammar`. Harper and the `[patch.crates-io]` stubs that keep a CUDA and LLVM subtree out of
the build. This crate is not in the list above because it was written before the audit; grammar is
neither typesetting nor macOS and needs its own home. See [typesetting.md](typesetting.md), which is
the authority on this group.

`margin-update`. The updater wiring and the release overlay convention.

## How an app depends on them

For npm, publish `@margin/*` to the registry and depend on them by semver range. It is the boring
option and it is the one that works on a fresh clone, in CI and on someone else's machine. The cost
is a publish step, which a `just release-shared` recipe and a CI job on tag reduce to one command.

The no-registry alternative, if publishing is unwanted, is a git subpath dependency, which pnpm
supports: `"@margin/tokens": "github:priyanshujain/margin-shared#<tag>&path:/packages/tokens"`. It
works, it needs no account, and it is less standard, so document it if it is chosen.

For local iteration on shared code, `pnpm.overrides` in the app's `package.json` pointing at a local
checkout, committed as a comment and not as a value, or `pnpm link`. The point is that the override is
the exception a developer turns on, not the default the repo ships.

For Rust, cargo git dependencies pinned to a tag are already idiomatic and need no registry at all:

    margin-google = { git = "https://github.com/priyanshujain/margin-shared", tag = "google-v0.3.0" }

For local iteration, a `[patch]` section or a `paths` entry in `.cargo/config.toml`, again as the
exception rather than the default.

## Versioning

One version for the whole shared repo, tagged `v0.3.0`, with every package and crate moving together.
Independent versioning of eleven packages consumed by four apps is a matrix nobody wants to reason
about, and these packages are not independent: a token rename breaks the UI kit.

The apps keep their own versions, which is what the App Store and the updater care about.
