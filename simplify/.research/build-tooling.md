# Build and developer tooling across the four apps

Scope: package.json, lockfiles, vite, tsconfig, index.html, justfiles, scripts, gitignore, nix,
editor config, Cargo profiles, capabilities. Not CI, signing or tests, bar where build leaks in.

Paths: margin `/Users/pj/Workspace/projects/python/margin`, margin-calendar
`/Users/pj/Workspace/projects/python/margin-caledar`, margin-docs
`/Users/pj/Workspace/projects/rust/margin-editor`, margin-mail
`/Users/pj/Workspace/projects/rust/margin-mail`. Cites below are relative to those roots.

## Five findings first

1. **margin-docs cannot install on a fresh clone or on its own CI.** Its `package.json:33` asks for
   `"margin-shared": "file:../../python/margin/shared"` but `.github/workflows/ci.yml:19` does one
   checkout. The frontend job dies inside `pnpm install --frozen-lockfile` with exit 254; four of
   the last five runs failed (run 33308997470, 2026-08-30: `rust: success`, `frontend: failure`).
   margin-mail is the only app that solved it, with a second checkout of `priyanshujain/margin`
   into `python/margin` (`.github/workflows/ci.yml:26-31`, `release.yml:109`).
2. **The four tsconfigs are three identical files plus one that differs by two lines.** md5 of
   margin, margin-calendar and margin-docs `tsconfig.json` is `468c4a26...`; margin-mail differs
   only in `target` and `lib`. All four `tsconfig.node.json` are byte identical (`767b2e9a...`).
3. **The three justfiles are one file with the product name swapped**, plus two extra recipes and a
   signing block in margin-mail. margin has no justfile at all, so the "every fix ends with
   `just install`" rule is unenforceable there.
4. **No prettier, no eslint, no biome, no .editorconfig, no rustfmt.toml, no rust-toolchain file in
   any of the four.** Confirmed by search over the repo roots and by grep over each package.json.
   The only editor config is `margin/.vscode/extensions.json`, two recommendations, and no sibling
   has one.
5. **Nix exists only in margin-calendar** and is a publishing artifact, not a toolchain: it
   repackages the released `.deb`. Worth copying per app, but not shared build config.

## package.json

### Scripts

| script | margin | calendar | docs | mail |
| --- | --- | --- | --- | --- |
| `dev` | `vite` | `vite` | `vite` | `vite` |
| `build` | `tsc && vite build` | same | same | same |
| `preview` | `vite preview` | same | same | same |
| `tauri` | `tauri` | same | same | same |
| `dmg` | `tauri build --bundles dmg` (`:11`) | absent | absent | absent |
| `test` | absent | `vitest run` | `vitest run` | `vitest run` |
| `test:watch` | absent | `vitest` | `vitest` | `vitest` |
| `test:ui` | absent | `playwright test` | `playwright test` | `playwright test` |
| `fonts:sync` | `node node_modules/margin-shared/bin/sync-fonts.mjs .` (`:12`) | absent | same (`:15`) | `margin-shared-fonts .` (`:15`) |
| `fonts:check` | same with `--check` (`:13`) | absent | same (`:16`) | `margin-shared-fonts . --check` (`:16`) |

Two drifts worth folding: margin and margin-docs invoke the font sync by path into `node_modules`,
margin-mail uses the `margin-shared-fonts` bin the package already declares
(`python/margin/shared/package.json:16-18`) and which is linked in all three consumers. The bin form
is the correct one. margin-calendar has no font sync and vendors four files under `public/fonts`
against eighteen in the others, so it is not on the shared face set.

`license` also drifts: `"SEE LICENSE IN LICENSE"` in margin (`:41`) and mail (`:5`), `"MIT"` in
calendar (`:5`) and docs (`:3`), matching `LicenseRef-FSL-1.1-MIT` and `MIT` respectively in the
Cargo manifests. Consistent within an app, but the four are not on one licence.

### Version drift (declared spec, then what the lockfile resolved)

| package | margin | calendar | docs | mail |
| --- | --- | --- | --- | --- |
| react, react-dom | `^19.1.0` -> 19.2.7 | `^19.1.0` -> 19.2.8 | 19.2.8 | 19.2.8 |
| vite | `^7.0.4` -> 7.3.5 | 7.3.6 | 7.3.6 | 7.3.6 |
| typescript | `~5.8.3` -> 5.8.3 | 5.8.3 | 5.8.3 | 5.8.3 |
| @vitejs/plugin-react | `^4.6.0` -> 4.7.0 | 4.7.0 | 4.7.0 | 4.7.0 |
| vitest | absent | `^3.2.4` -> 3.2.7 | 3.2.7 | 3.2.7 |
| @playwright/test | absent | 1.62.1 | 1.62.1 | 1.62.1 |
| zustand | `^5.0.14` -> 5.0.14 | 5.0.14 | 5.0.15 | 5.0.15 |
| @types/react | 19.2.17 | 19.2.18 | 19.2.18 | 19.2.18 |
| @types/react-dom | 19.2.3 | 19.2.4 | 19.2.4 | 19.2.7 |
| @types/node | absent | absent | `^22.20.1` -> 22.20.1 | `^24.0.0` -> 24.13.3 |
| @tauri-apps/api | `^2` -> 2.11.1 | 2.11.1 | 2.11.1 | 2.11.1 |
| @tauri-apps/cli | `^2` -> 2.11.3 | 2.11.4 | 2.11.4 | 2.11.4 |
| plugin-opener | 2.5.4 | 2.5.4 | 2.5.4 | 2.5.5 |
| plugin-process | 2.3.1 | 2.3.1 | 2.3.1 | 2.3.1 |
| plugin-updater | 2.10.1 | 2.10.1 | 2.10.1 | 2.11.0 |
| plugin-dialog | `^2.7.1` -> 2.7.1 | absent | `^2` -> 2.7.2 | absent |
| plugin-notification | absent | absent | absent | 2.4.0 |
| plugin-os | absent | absent | absent | 2.3.2 |
| tiptap | `^3.27.1` -> 3.27.1 | absent | `3.30.2` exact | `^3.31.2` -> 3.31.2 |

Nothing here is a real incompatibility. Every spec except margin-docs' tiptap is a caret or tilde,
so the drift is purely "when was `pnpm install` last run here": margin is the stale one, a patch
behind on react and vite and two `@types` bumps behind. The one deliberate difference is margin-docs
pinning tiptap exactly at 3.30.2 (`package.json:24-28`) while the others float.

`@types/node` is the only genuine split: 22 in docs, 24 in mail, absent in the other two. Since
`tsconfig.json` in all four sets no `types` array, the presence of `@types/node` silently changes
what global names typecheck per app.

No app declares a `packageManager` field, so nothing pins pnpm from the repo itself.

## The margin-shared relative path

- margin: `"margin-shared": "file:./shared"` (`package.json:26`), inside its own repo, and the
  directory is tracked (26 files under `shared/`).
- margin-docs: `"file:../../python/margin/shared"` (`package.json:33`).
- margin-mail: `"file:../../python/margin/shared"` (`package.json:27`).

The lockfiles record the literal relative string, with no integrity hash:
`rust/margin-mail/pnpm-lock.yaml:918` is `margin-shared@file:../../python/margin/shared:` with
`resolution: {directory: ../../python/margin/shared, type: directory}` and the snapshot at
`:1957` is `{}`. Same shape at `rust/margin-editor/pnpm-lock.yaml:1449`.

How fragile: pnpm resolves the path relative to the importer directory, so the dependency is not
"the margin repo", it is "two directories up, then `python/margin/shared`". That encodes PJ's local
grouping (`Workspace/projects/python`, `Workspace/projects/rust`) into a committed manifest.

- **Fresh clone.** Cloning margin-mail into `~/code/margin-mail` makes the target
  `/Users/pj/python/margin/shared`. `pnpm install` then stops with
  `ERR_PNPM_LINKED_PKG_DIR_NOT_FOUND  Could not install from "..." as it does not exist.`
  (reproduced directly, exit non-zero, nothing installed). This is not a warning that degrades to a
  missing font, it is a hard install failure before any other dependency lands.
- **CI.** margin-mail works only because `ci.yml:26-31` checks the sibling out at the exact path
  `python/margin` and runs everything with `working-directory: rust/margin-mail`. margin-docs does
  not do this and its frontend job has been red since the dependency landed.
- **Anyone else.** A contributor must clone two repositories into a two-level layout whose folder
  names (`python`, `rust`) mean nothing to them and appear in no documentation. The margin repo is
  public, so it is possible, just undiscoverable.
- **Reproducibility.** A directory dependency has no hash, so `pnpm install --frozen-lockfile`
  consumes whatever is in `shared/` at that moment, uncommitted edits included. The lockfile is not
  frozen with respect to shared code.
- **Publishing.** `shared/package.json:4` is `"private": true`, so today it cannot go to a registry
  without a deliberate change.

Three ways out, in order of how much they cost:

1. **Give shared its own repo and depend on a git tag.** Removes the path assumption entirely and
   gets an immutable resolution. npm and pnpm git dependencies cannot point at a subdirectory, so
   this means moving `shared/` out of the margin repo, which also fixes margin depending on it via
   `file:./shared`.
2. **Publish `margin-shared` to npm** (or a GitHub npm registry) and depend on a version. Same
   benefit, plus a real integrity hash in the lockfile. Costs a publish step per change to shared.
3. **Keep the relative path but make it discoverable and enforced:** an `.env`-style documented
   layout, a preinstall check that fails with a readable message instead of pnpm's error, and the
   second checkout added to margin-docs CI. This is the cheap fix and it leaves the reproducibility
   hole open.

If a shared toolchain package is going to exist anyway, it should be delivered the same way as
whatever is chosen here, and the two should not use different mechanisms.

## pnpm and workspaces

All four lockfiles are `lockfileVersion: '9.0'` (line 1) with identical settings blocks
(`autoInstallPeers: true`, `excludeLinksFromLockfile: false`). No `pnpm-workspace.yaml` and no
`.npmrc` in any of the four. There is no workspace today and no way to create one across four git
repos without either submodules or a monorepo merge.

Local installs all report `packageManager: pnpm@10.12.4` in `node_modules/.modules.yaml`, which is
install state rather than a committed pin; CI pins `pnpm/action-setup@v6` `version: 10` and node 26.

## vite.config.ts

Ports, which are the one thing that must stay per app and are correctly staggered:

| app | server.port | hmr.port | tauri devUrl |
| --- | --- | --- | --- |
| margin | 1420 (`:17`) | 1421 (`:23`) | `http://localhost:1420` |
| calendar | 1430 (`:13`) | 1431 (`:20`) | `http://localhost:1430` |
| docs | 1440 (`:23`) | 1441 (`:30`) | `http://localhost:1440` |
| mail | 1450 (`:16`) | 1451 (`:22`) | `http://localhost:1450` |

Everything else in the file is the same four properties: `plugins: [react()]`,
`clearScreen: false`, `strictPort: true`, `host: host || false` where `host` is
`process.env.TAURI_DEV_HOST` behind a `@ts-expect-error` comment in all four (`:4-5` in each), the
same conditional `hmr` block, and `watch.ignored`.

Real differences:

- margin imports `defineConfig` from `"vite"` (`:1`) and exports an async factory,
  `defineConfig(async () => ({ ... }))` (`:8`), for no reason visible in the file. The other three
  import from `"vitest/config"` and export a plain object, because they carry a `test` block.
- margin ignores `"**/website/**"` as well as src-tauri (`:29`); the others ignore only src-tauri.
- margin-docs is the only one with a `build` block: `assetsInlineLimit` as a function that returns
  `false` for `woff2?|ttf|otf|eot` (`:19`), because the app CSP is `font-src 'self'` and a data URI
  font would be refused.
- The `test` blocks: calendar and mail are identical (`include: ["src/**/*.test.ts"]`,
  `environment: "node"`); docs adds `maxWorkers: "50%"`, `testTimeout: 30_000`,
  `hookTimeout: 30_000`, `teardownTimeout: 30_000` (`:57-69`).

Nothing in any of the four sets `define`, `envPrefix`, `resolve.alias`, `build.target`, `minify` or
`sourcemap`. So there is no alias story to preserve and no env prefix convention to standardise.

## tsconfig.json and tsconfig.node.json

`tsconfig.json` is identical in margin, calendar and docs. margin-mail differs in exactly two
options:

- `"target": "ES2022"` versus `"ES2020"` (`rust/margin-mail/tsconfig.json:3`)
- `"lib": ["ES2022", "DOM", "DOM.Iterable"]` versus `["ES2020", ...]` (`:5`)

Everything else matches across all four: `useDefineForClassFields`, `module: "ESNext"`,
`skipLibCheck`, `moduleResolution: "bundler"`, `allowImportingTsExtensions`, `resolveJsonModule`,
`isolatedModules`, `noEmit`, `jsx: "react-jsx"`, `strict`, `noUnusedLocals`, `noUnusedParameters`,
`noFallthroughCasesInSwitch`, `include: ["src"]`, and a reference to `./tsconfig.node.json`.

`tsconfig.node.json` is byte identical in all four: `composite`, `skipLibCheck`, `module: ESNext`,
`moduleResolution: bundler`, `allowSyntheticDefaultImports`, `include: ["vite.config.ts"]`.

`tests/tsconfig.json` exists in calendar, docs and mail (margin has no tests directory). Calendar
and docs are byte identical. margin-mail adds `"types": ["node"]` (`:16-18`) and is written with
one array element per line, which is a formatting drift nothing enforces.

Nothing runs `tests/tsconfig.json`. `pnpm build` is `tsc && vite build`, and root `tsc` only sees
`include: ["src"]`. No package.json script, justfile recipe or workflow step in any of the three
references it. The Playwright specs are therefore type checked by nobody.

None of the four sets `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
`verbatimModuleSyntax` or `paths`.

## index.html

Identical structure in all four: `<!doctype html>`, `lang="en"`, `charset=UTF-8`, a title, a
`<div id="root">`, and `<script type="module" src="/src/main.tsx">`.

- Viewport: margin, calendar and mail use `width=device-width, initial-scale=1.0,
  maximum-scale=1.0, user-scalable=no, viewport-fit=cover`; docs drops `viewport-fit=cover` (`:5`).
- Favicon: `/margin-mark.png`, `/margincal-mark.png`, `/marginmail-mark.png`. margin-docs has no
  `<link rel="icon">` at all and no mark in `public/`.
- No CSP meta tag and no font preload in any of the four. The real CSP is in
  `src-tauri/tauri.conf.json` under `app.security.csp`, and it differs per app for good reasons:
  margin and docs add `worker-src 'self' blob:`, docs adds `asset: http://asset.localhost` to
  `img-src`, mail adds `frame-src 'self'`.
- Every one has an inline pre-paint script that reads localStorage and sets attributes on
  `documentElement` before React boots. They share a shape (theme resolution against
  `prefers-color-scheme`, then app specific attributes) but every key is app prefixed
  (`margin-theme`, `margincal-theme`, `marginmail-theme`, `margindocs-theme`) and each sets
  different things. margin-docs is the only one that reads keys individually rather than wrapping
  the lot in one `try` (`:13-19`), which is the better version: a webview that refuses storage
  still gets the theme. The other three lose everything after the first throw.

That inline script is the one genuinely shared idea in these files, and it is the hardest to share,
because it must be inline and cannot import.

## justfiles

margin has none. calendar, docs and mail have one, and the three are the same file with names
substituted; `diff` between calendar and docs is nine hunks, all of them the product name, the
process name or a comment reflow.

| recipe | calendar | docs | mail | bodies match |
| --- | --- | --- | --- | --- |
| `default` (`@just --list`) | yes | yes | yes | identical |
| `dev` (`pnpm tauri dev`) | yes | yes | yes | identical |
| `test` (`pnpm test` then `cd src-tauri && cargo test`) | yes | yes | yes | identical |
| `test-ui` (`pnpm test:ui`) | yes | yes | yes | identical, comment differs in mail |
| `guide-shots` | no | no | yes (`:29-30`) | mail only |
| `docs` (`node scripts/docs-check.mjs`) | no | no | yes (`:33-34`) | mail only |
| `build` | yes | yes | yes | same except mail's signing block |
| `install` (depends on `build`) | yes | yes | yes | identical |
| `_install-macos` | yes | yes | yes | docs omits the final `open "$dest"` |
| `_install-linux` | yes | yes | yes | identical bar names and the desktop entry |
| `uninstall` | yes | yes | yes | identical bar names |

Shared variables: `set shell := ["bash", "-euo", "pipefail", "-c"]`, `app`, `bundle :=
"src-tauri/target/release/bundle"`.

Substantive differences:

- margin-mail's `build` sources a signing env file before building
  (`rust/margin-mail/justfile:46-54`): it reads
  `"${MARGIN_SIGNING_DIR:-$HOME/.margin-signing}/studio.margin.app.env"`. Note the file name is
  `studio.margin.app`, which is margin's bundle identifier, not `studio.margin.mail`. If the intent
  is one env file for the whole suite the name is misleading; if it is one per app, this is wrong.
- margin-docs' `_install-macos` does not `open "$dest"` at the end (compare `justfile:77` in docs
  with `:78` in calendar and `:97` in mail). So `just install` starts the new build in two of the
  three apps and not the third.
- Only margin-mail has a prose gate recipe.

Because the rule is that a fix ends with `just install`, the gap that matters is margin: its only
local build path is `pnpm dmg` (`package.json:11`), and nothing copies a bundle into
`/Applications`. The three justfiles already prove the body is app independent bar four names, so
margin is a paste plus a variable block away from the same command.

## scripts directories

Only two apps have one and they do unrelated things.

- `margin/scripts` is twelve files of Apple release plumbing: `apple-provision.rb`,
  `apple-secrets.sh`, `appstore-compliance.rb`, `appstore-listing.rb`,
  `appstore-review-detail.rb`, `appstore-screenshots.rb`, `mas-package.sh`, `mas-upload-local.sh`,
  `testflight-release.rb`, `testflight-setup.rb`, `testflight-testers.rb`. This is the App Store
  path and only margin is on it, so it stays where it is (and overlaps the signing agent's scope).
- `margin-mail/scripts` is one file, `docs-check.mjs`, 72 lines: no em or en dash anywhere
  including in code fences, no directory tree (box glyph run or three consecutive ASCII tree
  lines), no broken relative link. It walks every `.md` in the repo, skipping
  `node_modules, dist, target, .git, gen, .playwright-mcp`.

`docs-check.mjs` is the clearest single candidate for sharing: no dependencies, no app specific
knowledge, and it enforces a house rule that applies to all four repos. It belongs in the shared
package with a bin, like `sync-fonts.mjs` (exposed as `margin-shared-fonts`).

## .gitignore

Common core in all four, in the same order: log patterns, `node_modules`, `dist`, `dist-ssr`,
`*.local`, the editor block (`.vscode/*` with `!.vscode/extensions.json`, `.idea`, `.DS_Store`,
`*.sw?`), `src-tauri/target/`, `.playwright-mcp/`, `/*.png`.

Differences:

- `src-tauri/gen/schemas/` is ignored in calendar (`:27`), docs (`:27`) and mail (`:21`), but not
  in margin. margin therefore can commit generated schema files.
- The Xcode block (`src-tauri/gen/apple/build/`, `Externals/`, `Pods/`, `Podfile.lock`,
  `xcuserdata/`) is in calendar (`:32-36`), docs (`:32-36`) and mail (`:25-29`), not margin.
- Google credentials (`/google-credentials.json`, `/client_secret_*.json`) in margin (`:30-31`),
  calendar (`:39-40`) and mail (`:33-34`), not docs, which needs none.
- margin only: `target-mas/` and two App Store review contact files (`:38-42`).
- docs only: `.env`, `.env.*`, `!.env.example` (`:43-45`), the updater signing key.
- calendar only: `/result`, `/result-*` (`:47-48`), the nix build symlinks.
- mail only: `/screenshots/` (`:41`).
- margin-mail trims the editor block hardest (no `*.suo`, `*.ntvs*`, `*.njsproj`, `*.sln`).

A shared base of about twenty lines would cover everything up to `/*.png`, with five to eight app
specific lines after it. Git has no include mechanism for ignore files and `core.excludesFile` is
per machine, so this is one of the things that stays copied.

## Nix in margin-calendar

`flake.nix` is 21 lines: one input (`nixpkgs` at `nixos-unstable`, locked in `flake.lock` at rev
`3ed67ec0a4d3c7ab4ae1f04f8ee8df07bfa506a2`), an overlay and a single `x86_64-linux` package that
calls `./nix/package.nix`. `nix/package.nix` fetches the published `.deb` from the GitHub release
named in `nix/release.json` (`{"version": "0.0.5", "hash": "sha256-+bEQO..."}`), unpacks it with
`dpkg-deb -x`, relinks it with `autoPatchelfHook` and `wrapGAppsHook3` against nixpkgs' gtk3,
`webkitgtk_4_1`, `libsoup_3` and friends, writes a launcher that points `libglvnd` at nixpkgs' mesa
when `/run/opengl-driver` is absent, shims `xdg-open` to strip those variables again, and sets
`MARGIN_CALENDAR_PACKAGED_BY=nix` so the in-app updater reports rather than replaces itself.

`docs/release.md:41-81` gives the rationale: the AppImage bundles Ubuntu's GTK stack and a bundled
`libwayland-client` cannot talk to a current compositor, so on Hyprland it falls back to Xwayland.
It is binary by necessity, because the Google OAuth client is embedded at compile time from a file
that is not in the repo.

Is this the Linux distribution path, and should the others adopt it? Yes for margin-mail, same
Google client problem and same GTK and WebKit runtime; yes for margin-docs if it ships Linux, which
its justfile already builds for. margin is macOS and App Store shaped and would gain nothing.

But note what is actually shared here: almost nothing. The flake is fifteen lines of boilerplate and
`package.nix` is a hundred lines of which the app name, the deb URL, the desktop file rename, the
env variable name and the meta block are all per app, and the rest (the hooks, the buildInputs list,
the mesa launcher, the xdg-open shim) is genuinely common. If two apps adopt it, that common part
should be a function in the shared repo that each flake calls with a name, a repo and a release pin.
Below two adopters, copy it.

## Editor, formatter and linter config

Confirmed absent everywhere: prettier (no config file, no dependency, no script in any of the four
package.json files), eslint, biome, `.editorconfig`, `rustfmt.toml`, `clippy.toml`,
`rust-toolchain.toml`, `.nvmrc`.

Present: `margin/.vscode/extensions.json`, two recommendations
(`tauri-apps.tauri-vscode`, `rust-lang.rust-analyzer`). No other app has a `.vscode` directory,
though all four gitignore `.vscode/*` while un-ignoring `extensions.json`, so the intent is there.

`margin-editor/src-tauri/.cargo/config.toml` is the only cargo config: `[env] RUST_TEST_THREADS =
"1"`, for a suite that shares one on-disk git repository. App specific, stays.

The consistent formatting across all these files (two space JSON, 100 column comments, the same
comment voice) is being maintained by hand. That works while one person writes everything, and the
one place it has already slipped is `rust/margin-mail/tests/tsconfig.json`, which is the same file
as its siblings reformatted with expanded arrays.

## Cargo

No `[workspace]` section in any of the four manifests, so each `src-tauri` is its own workspace root
with its own `Cargo.lock` (margin 964 packages, calendar 565, docs 962, mail 668).

Profiles: only margin (`src-tauri/Cargo.toml:78-79`) and margin-docs (`:107-108`) set anything,
both `[profile.dev.package."*"] opt-level = 3` with near identical comments about Harper's grammar
engine being ten times slower unoptimized. No `[profile.release]` anywhere, so release builds are
cargo defaults in all four and there is no `lto`, `codegen-units` or `strip` setting to align.

Duplication worth noting: margin and margin-docs both carry `[patch.crates-io]` stubs for
`burn-cuda` and `cubecl-cpu` (`margin:70-72`, `docs:98-100`) plus a `stubs/` directory each with the
same two skeleton crates at the same versions (`burn-cuda` 0.19.1, `cubecl-cpu` 0.8.1) and nearly
the same comments. This is real shared code, kept in sync by hand, and it is tied to
`harper-core = "=2.5.0"` in both.

A shared cargo workspace across the four is not feasible. A workspace requires one filesystem root
containing all members with paths in the root manifest, which means one git repository. These are four
repositories with independent release tags and version numbers, and margin-mail has no remote
configured yet. Without merging, the one thing worth extracting is a shared crate for the harper
stubs behind a git dependency, which deletes both copied `stubs/` directories.

## Capabilities

All four have exactly `capabilities/default.json` and `capabilities/desktop.json`, both pointing at
`../gen/schemas/desktop-schema.json`, both scoped to `windows: ["main"]`, with desktop gated on
`platforms: ["macOS", "windows", "linux"]`. No `permissions/` directory in any app.

| app | default permissions | desktop permissions |
| --- | --- | --- |
| margin | `core:default`, `core:window:allow-destroy`, `core:window:allow-start-dragging`, `opener:default`, `dialog:default` | `updater:default`, `process:allow-restart` |
| calendar | `core:default`, `opener:default`, `deep-link:default` | the two window permissions, `updater:default`, `process:allow-restart` |
| docs | `core:default`, `opener:default`, `dialog:default` | the two window permissions plus `core:window:allow-toggle-maximize`, `updater:default`, `process:allow-restart` |
| mail | `core:default`, `opener:default`, `deep-link:default`, `notification:default`, `os:default` | the two window permissions, `updater:default`, `process:allow-restart` |

margin is the odd one: it puts the two window permissions in `default` rather than `desktop`, so a
mobile build would ask for them. `desktop.json` is identical in calendar and mail bar the
description, and docs differs by one line.

## Recommendation

A shared package (call it `margin-shared` extended, or a second `margin-config` delivered the same
way) should export exactly five things:

1. **`tsconfig/base.json`.** Everything currently duplicated, with `target` and `lib` at ES2022 for
   all four, since ES2020 in three of them is a scaffold default nobody chose. Each app keeps a
   three line `tsconfig.json` that extends it and sets `include` and `references`. Also export
   `tsconfig/node.json` (byte identical in all four today) and `tsconfig/tests.json` (identical in
   two of three, one `types` entry apart). Adding `noUncheckedIndexedAccess` is a separate decision
   and should not ride along with the consolidation.
2. **A vite config factory**, `marginVite({ port, test })`, returning the plugin, `clearScreen`,
   the full `server` block derived from one port number, and the `TAURI_DEV_HOST` handling. Ports
   stay per app and are the argument. margin-docs passes its `assetsInlineLimit`, margin-docs
   passes its vitest timeouts. The `@ts-expect-error process` comment disappears with it, because
   the factory can own that line once.
3. **A justfile include.** `just` supports `import`, so the shared file can hold `default`, `dev`,
   `test`, `test-ui`, `build`, `install`, `_install-macos`, `_install-linux` and `uninstall`
   verbatim, parameterised on `app`, `binary`, `comment` and `categories`, which the app justfile
   sets before importing. Adding this to margin is the change that makes `just install` a real
   suite-wide rule instead of a rule three of four apps can honour. The import has to resolve to a
   real path, which lands back on the same delivery question as `margin-shared`.
4. **`docs-check` as a bin.** Move `margin-mail/scripts/docs-check.mjs` into the shared package
   beside `sync-fonts.mjs`, expose it as `margin-shared-docs`, and add a `docs` recipe to the shared
   justfile. It has no app specific content at all.
5. **The checking story, with no eslint.** Today that is `tsc` under `pnpm build` plus `cargo
   test`. Two gaps close inside the shared config rather than with a linter: `tests/tsconfig.json`
   is run by nothing, so add a `typecheck` recipe covering both projects; and `pnpm fonts:check`
   runs in margin-mail CI only, so it belongs in the shared `test` recipe everywhere.

What has to stay per app, and should not be abstracted:

- The dev server port and the matching `devUrl` in `tauri.conf.json`. Staggering is load bearing:
  Playwright reuses whatever answers on the port, so a collision means a suite silently driving the
  wrong app (`rust/margin-mail/vite.config.ts:7-9`).
- The inline pre-paint script in `index.html`. It cannot import, its storage keys are app prefixed,
  and it sets different attributes per app. Copy margin-docs' per key `saved()` pattern into the
  other three by hand.
- The CSP in `tauri.conf.json`, the capabilities files and `Cargo.toml` dependencies. One shared
  version would be the union, which is wrong for a suite that does not reach for spare permissions.
- `.gitignore`, because git cannot include a shared file.
- The nix flake, unless and until a second app ships Linux through it.
- margin's `scripts/` App Store tooling, and margin-docs' `src-tauri/.cargo/config.toml`.

Sequencing: the `margin-shared` path problem has to be solved first, because every item above is
delivered through the same mechanism, and adding four more consumers of a broken relative path makes
the fresh clone failure four times worse instead of once. Second, add the missing checkout to
margin-docs CI, which is a red build today for a reason nobody has looked at. Third, give margin a
justfile.
