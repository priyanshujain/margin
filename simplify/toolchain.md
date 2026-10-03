# Toolchain and the developer loop

What `@margin/config` contains, how four repositories run the same gate and the same `just install`, and
which of today's arrangements are broken rather than merely duplicated.

## The relative path is a broken build, not untidiness

Margin Docs asks for `"margin-shared": "file:../../python/margin/shared"`
(`rust/margin-editor/package.json:33`) and Margin Mail asks for the same string
(`rust/margin-mail/package.json:27`); Margin's own `file:./shared` (`python/margin/package.json:26`) is
inside its repository and fine. pnpm resolves the first two relative to the importer, so the dependency
is not "the Margin repository", it is "two directories up, then `python/margin/shared`", which encodes
one laptop's folder layout in a committed manifest.

There are two failure modes with two different errors. A fresh clone with no lockfile, reproduced here
on pnpm 10.12.4:

    ERR_PNPM_LINKED_PKG_DIR_NOT_FOUND  Could not install from "/tmp/.../python/margin/shared" as it
    does not exist.

    This error happened while installing a direct dependency of /tmp/.../rust/app

With the lockfile committed, `--frozen-lockfile` skips resolution and gets further before dying, which
is what CI hits. Margin Docs run 33308997470 (2026-08-30, push to main), job `frontend`, step
`pnpm install --frozen-lockfile`:

     ENOENT  ENOENT: no such file or directory, scandir '/Users/runner/work/python/margin/shared'
    ##[error]Process completed with exit code 254.

`pnpm build` and `pnpm test` are then skipped, so that repository has had no typecheck and no frontend
test in CI at all. Six of the last seven runs failed exactly there, and the `rust` job was green in every
one, which is why it has gone unnoticed. Margin Mail escapes only because
`rust/margin-mail/.github/workflows/ci.yml:27-31` checks `priyanshujain/margin` out a second time at
`python/margin` and runs everything with `working-directory: rust/margin-mail`.

The second half is quieter and worse. A directory dependency carries no integrity hash; the lockfile
entry at `rust/margin-mail/pnpm-lock.yaml:918` is

    margin-shared@file:../../python/margin/shared:
      resolution: {directory: ../../python/margin/shared, type: directory}
      hasBin: true

with an empty snapshot at `:1957`, and Margin Docs has the same shape at `:1449` and `:3207`. So
`pnpm install --frozen-lockfile` installs whatever bytes are in `shared/` at that moment, uncommitted
edits included. The lockfile is frozen with respect to every dependency except the one that is being
actively edited, which is the inversion of what a lockfile is for.

## The three ways out

| | Fresh clone works | Immutable | Cost per shared change |
|---|---|---|---|
| Own repo, cargo-style git dependency pinned to a tag | yes | yes, commit hash in the lockfile | tag, then bump four manifests |
| Publish `@margin/*` to a registry, depend by semver | yes | yes, integrity hash | publish, then bump four manifests |
| Keep the path, add a preinstall guard and the missing CI checkout | no | no | none |

Publish, which agrees with [repo-layout.md](repo-layout.md) and is right: it is the only option where
`pnpm install` on a machine that has never heard of this suite does the correct thing with no
instructions, and the only one that puts a hash next to shared code in the lockfile. The npm git-subpath
form works (`github:priyanshujain/margin-shared#<tag>&path:/packages/config`) and is the fallback if a
registry account is unwanted, at the cost of being the thing nobody else does. The third option is not a
way out at all: it leaves the reproducibility hole open by construction, and this consolidation ends with
four repositories consuming shared code, which turns one fresh-clone failure into eight.

Two consequences. `python/margin/shared/package.json:4` is `"private": true`, so publishing means
removing that and renaming into the `@margin` scope. And `@margin/config` has to arrive the same way as
everything else, because both things it delivers resolve through `node_modules`: the tsconfig through
`extends`, the justfile through `import?`.

## `@margin/config`

### The base tsconfig

Three of the four `tsconfig.json` files are byte identical and Margin Mail differs in two lines
(`target` and `lib` at ES2022 against ES2020). Which per-app options legitimately remain? None. ES2020
in three of them is a Vite scaffold default nobody chose, Margin Mail already moved off it, and all four
build for the same Tauri webview. The base sets ES2022 and no app overrides it; if a real difference
ever appears, `compilerOptions` in the app config still wins.

`@margin/config/tsconfig/base.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "useDefineForClassFields": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "skipLibCheck": true,
    "types": [],
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  }
}
```

The one addition to today's set is `"types": []`, which stops whatever `@types/*` packages happen to be
installed from becoming ambient globals in webview code; the versions section is about why that matters.
I checked it against all four `src` trees with each app's own compiler and every one is clean, so it
costs nothing to adopt. `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` and
`verbatimModuleSyntax` stay off: each is a real decision with real diffs behind it and none of them
should ride in on a consolidation.

The app's whole `tsconfig.json` becomes two lines:

```json
{ "extends": "@margin/config/tsconfig/base.json", "include": ["src"] }
```

`extends` resolves a scoped package subpath through the `exports` map, verified on TypeScript 5.8.3.
Relative paths resolve against the file that declares them, which is why `include` stays in the app.

`tsconfig.node.json`, byte identical in all four, is replaced by `@margin/config/tsconfig/tools.json`
plus a two-line app file including `vite.config.ts`, `playwright.config.ts` and `tests`. Two changes from
today: `types: ["node"]`, because those files really are Node, and `composite` goes away with the
`references` array. `composite` is why nothing checks that project (plain `tsc` never builds references),
and running `tsc -p tsconfig.node.json` by hand drops a `tsconfig.node.tsbuildinfo` in the repo root,
which no `.gitignore` in the suite covers. I did that to all four repos while checking this and had to
clean up after myself.

### The Vite factory

```ts
export function marginVite(app: { name: string; port: number }, extra: UserConfig = {}): UserConfig
```

The app passes its name and its dev server port and nothing else:

```ts
import { marginVite } from "@margin/config/vite";
export default marginVite({ name: "Margin Mail", port: 1450 });
```

The factory derives the HMR port as `port + 1`, which is the convention all four already follow
(1420/1421, 1430/1431, 1440/1441, 1450/1451), and owns for everyone: `plugins: [react()]`,
`clearScreen: false`, `strictPort: true`, `host` from `TAURI_DEV_HOST`, the conditional `hmr` block,
`watch.ignored: ["**/src-tauri/**"]`, and the vitest defaults `include: ["src/**/*.test.ts"]` and
`environment: "node"`. Margin's pointless `defineConfig(async () => ({ ... }))` wrapper
(`python/margin/vite.config.ts:8`) goes with it, as does the `// @ts-expect-error process is a nodejs
global` line that appears at `:4` in all four and is wrong in two of them (see the gate below).

`name` earns its place by feeding a dev-server plugin that answers `/__margin` with the app name, so
`@margin/test`'s Playwright config can refuse to run against a sibling's server. Every playwright config
in the suite sets `reuseExistingServer: true`, and the staggered ports are all that stand between that
and a suite driving the wrong app. Margin Docs' `tests/identity.ts` keeps its byte comparison of five
witness files on top, because a name cannot catch a stale server running the same app.

`extra` is merged with Vite's `mergeConfig`, and there are exactly two users of it today: Margin Docs'
`build.assetsInlineLimit` (`rust/margin-editor/vite.config.ts:19`, fonts must never inline because the
CSP is `font-src 'self'`) and its four vitest timeouts (`:57-69`).

## The justfile

| recipe | margin | calendar | docs | mail |
|---|---|---|---|---|
| `default`, `dev`, `test`, `test-ui` | absent | yes | yes | yes |
| `build` | absent | yes | yes | yes, plus a signing block |
| `install`, `_install-macos`, `_install-linux`, `uninstall` | absent | yes | yes, one line short | yes |
| `guide-shots` | absent | absent | absent | yes (`:29-30`) |
| `docs` | absent | absent | absent | yes (`:33-34`) |

Bodies are identical across the three that exist, bar the product name, the process name and comment
reflow. Margin has no justfile, so the rule that a task ends with `just install` is one three of four
apps can honour; its only local build path is `pnpm dmg` (`python/margin/package.json:11`), which copies
nothing into `/Applications`. Three repositories share one recipe file through an optional import:

```just
set shell := ["bash", "-euo", "pipefail", "-c"]
app := "Margin Mail"
binary := "margin-mail"
comment := "A calm, keyboard-first mail client for Gmail"
categories := "Office;Email;"
bundle := "src-tauri/target/release/bundle"
import? 'node_modules/@margin/config/just/app.just'
```

I tested this on just 1.49.0, including through a pnpm-style symlink into the store: variables set in
the importing file are visible to imported recipes, `set shell` applies to them, they run in the
importing justfile's directory (so `pnpm install` and `cd src-tauri` behave), and a missing import
degrades to listing the local recipes instead of erroring. That last property is why `import?` rather
than `import`: before the first `pnpm install` there is no `node_modules`, and a hard import would fail
`just --list` on a fresh clone with a parse error about a file the reader has never heard of. The app
keeps a local `setup: pnpm install` recipe for that first run.

Two bugs found in the files as they stand.

**Margin Mail sources Margin's signing environment file.** `rust/margin-mail/justfile:47` reads
`"${MARGIN_SIGNING_DIR:-$HOME/.margin-signing}/studio.margin.app.env"`. `studio.margin.app` is Margin's
bundle identifier (`python/margin/src-tauri/tauri.conf.json:5`); Margin Mail's is `studio.margin.mail`.
The file exists on this machine and holds `APPLE_TEAM_ID`, `APPLE_SIGNING_IDENTITY`, `MAS_APP_IDENTITY`
and `MAS_INSTALLER_IDENTITY`, all of which are team-wide rather than per app, so the build works today by
accident. It is still wrong twice over: the name says this is Margin's file, and Margin Mail's own
`docs/release.md:25` repeats the wrong name to the reader. One env file for the suite is the right
model, so rename it to `~/.margin-signing/apple.env` and have the shared `build` recipe read that, which
also means the other three apps get signed bundles the day they want them.

**Margin Docs never starts the app it just installed.** `_install-macos` ends at
`rust/margin-editor/justfile:77` with `echo "Installed $version to $dest"`. Margin Calendar has
`open "$dest"` at `:78` and Margin Mail at `:97`. The recipe quits the running app before replacing the
bundle, so in Margin Docs `just install` leaves the user with no app running and no sign anything
happened. Since the whole reason `just install` ends every task is that the user tests the installed app,
this is the one bug here with a behavioural cost.

The shared file holds `default`, `dev`, `check`, `test`, `test-ui`, `docs`, `build`, `install`,
`_install-macos`, `_install-linux` and `uninstall`. `guide-shots` stays in Margin Mail: committed
screenshots are its problem alone.

## The checking gate

Today the gate is `tsc` over `include: ["src"]` (as the first half of `pnpm build`), `vitest run`, and
`cargo test`, which subsumes `cargo check`. Playwright is run by hand. Three holes.

**Nothing type checks the Playwright specs.** `tests/tsconfig.json` exists in Margin Calendar, Margin
Docs and Margin Mail, and no package script, justfile recipe or workflow step in any of the three
mentions it. Not a theoretical gap: running each app's own compiler over it now gives

| app | result |
|---|---|
| Margin Mail | clean, 25 files |
| Margin Docs | 3 errors: `tests/_audit.spec.ts:4` and `_audit2.spec.ts:4` TS6133 unused `putCaret`, `_audit3.spec.ts:68` TS2698 spread of a non-object |
| Margin Calendar | 2 errors: `tests/bugs.spec.ts:23` and `tests/touch.spec.ts:42` TS2322, a `string` where the touch event union was wanted |

`vite.config.ts` is unchecked for the same reason and hides a real error there. `tsc -p
tsconfig.node.json` fails in Margin Mail and Margin Docs with `vite.config.ts(4,1): error TS2578: Unused
'@ts-expect-error' directive.` and passes in Margin and Margin Calendar, because the two with
`@types/node` installed already have `process` typed. That is the version split below, showing up as a
compiler error nobody can see.

**The font check runs in one app.** `pnpm fonts:check` is in Margin Mail's CI at
`rust/margin-mail/.github/workflows/ci.yml:44` and nowhere else. Margin (`package.json:13`) and Margin
Docs (`package.json:16`) both have the script and neither ever runs it. All three pass right now, 18
files matching, so wiring it in is free. Margin Calendar has neither the script nor the faces (4 files in
`public/fonts` against 18), so it joins when it moves onto `@margin/fonts`.

**The prose gate exists in one repo and only reads markdown.** `rust/margin-mail/scripts/docs-check.mjs`
is 72 lines, no dependencies, no app-specific knowledge. Run over the other three: Margin Calendar clean,
Margin 7 hits in `website/README.md`, Margin Docs 54 of which 53 are its deliberately malformed markdown
corpus under `src/markdown/corpus/`, the 54th a false positive at `docs/architecture.md:151` where prose
about markdown link syntax inside backticks is followed as a link. So it needs three changes on the way
into `@margin/config`: take the repo root as an argument the way `margin-shared-fonts .` does (it derives
it from its own location today, which would scan the package), take a skip list for fixture corpora, and
ignore inline code for links but not for dashes. It should also stop being markdown-only, because Margin
ships 4 em dashes in user-visible copy (`src/components/Library.tsx:88`,
`src/components/ExportPreview.tsx:185`, `src/export/run.ts:8`, `src/export/run.ts:36`) that no gate looks
at, while Margin Mail's `src/screens/guide/guide.test.ts:104` already asserts the rule over its guide
copy, which is the pattern to generalise.

The gate every app runs, behind one recipe name, `just check`:

```just
check:
    pnpm exec tsc -p tsconfig.json
    pnpm exec tsc -p tsconfig.tools.json
    pnpm exec margin-docs-check .
    pnpm exec margin-shared-fonts . --check
    pnpm test
    cd src-tauri && cargo test
```

`just test` stays as the inner loop, vitest plus `cargo test` and nothing else, because that is what you
run twenty times an hour. `just check` is what CI runs and what a task runs before handing back, ahead of
`just install`, which is still the last step.

## Dependency versions

Declared spec, then what the lockfile resolved:

| package | margin | calendar | docs | mail |
|---|---|---|---|---|
| react, react-dom | 19.2.7 | 19.2.8 | 19.2.8 | 19.2.8 |
| vite | 7.3.5 | 7.3.6 | 7.3.6 | 7.3.6 |
| typescript | 5.8.3 | 5.8.3 | 5.8.3 | 5.8.3 |
| vitest | absent | 3.2.7 | 3.2.7 | 3.2.7 |
| @playwright/test | absent | 1.62.1 | 1.62.1 | 1.62.1 |
| zustand | 5.0.14 | 5.0.14 | 5.0.15 | 5.0.15 |
| @types/react | 19.2.17 | 19.2.18 | 19.2.18 | 19.2.18 |
| @types/node | absent | absent | `^22.20.1` to 22.20.1 | `^24.0.0` to 24.13.3 |

Margin is the stale one: a patch behind on react and vite, two `@types` bumps behind, and the only app
with no `test` script, no vitest and no Playwright. Every spec except Margin Docs' exact tiptap pin at
3.30.2 is a caret or a tilde, so most of this table is "when was `pnpm install` last run here" rather than
a decision.

`@types/node` is the split that changes behaviour, and the gate section has the receipt. No
`tsconfig.json` in the suite sets `types`, so TypeScript picks up every `@types` package it finds: in
Margin Docs and Margin Mail `process`, `Buffer` and Node's `setTimeout` are ambient in webview source, in
Margin and Margin Calendar they are not, and the same `vite.config.ts` therefore compiles in two apps and
fails in two over a devDependency nobody thought of as load bearing. `"types": []` in the base ends it
for `src`, `"types": ["node"]` in the tools config puts the Node globals where they belong, and
`@types/node` becomes a devDependency in all four at one major, 24.

The policy for holding four repositories on one set of versions without a workspace. The specs stay in
each app's `package.json`, because pnpm links `vite`, `tsc`, `vitest` and `playwright` into
`node_modules/.bin` from direct dependencies only, and a version inherited through `@margin/config`
would not give the app a binary to run. What changes is that they stop being maintained by hand:
`@margin/config` owns the canonical list as `versions.json` and ships `margin-deps-check`, which
compares an app's `package.json` against it and fails with the lines that differ, run inside
`just check`. That is the same shape as `sync-fonts --check`, which the suite already trusts for the
font binaries, so it needs no new concepts and no registry cleverness. Upgrades land in the shared repo
first, then a Renovate preset held there
(`github>priyanshujain/margin-shared//renovate/default.json5`) opens one grouped toolchain PR per app.
Renovate bumping an app before the shared list moves is the failure to avoid, and it is what the check
catches.

Two pins nothing sets today. No app declares `packageManager`, so pnpm is whatever the person or the
runner happens to have (10.12.4 locally, `pnpm/action-setup@v6` `version: 10` in CI, which resolved to
10.34.5 in the Margin Docs run above). Add `"packageManager": "pnpm@10.12.4"` and a `.nvmrc` to all four,
then have the workflows read `node-version-file: .nvmrc` rather than repeating `node-version: 26` four
times while the machine that writes the code runs 25.5.0.

## Formatting and linting

There is none. No prettier, eslint or biome anywhere: no config file, no dependency, no script in any of
the four `package.json` files. No `.editorconfig`, no `rustfmt.toml`, no `clippy.toml`, no
`rust-toolchain.toml`. The only editor config in the suite is `python/margin/.vscode/extensions.json`,
two recommendations.

This is deliberate and it stays. The source is hand-formatted at roughly 120 columns and no formatter
config reproduces that, so `prettier --write` reflows at 80 and turns a 60-line change into a 340-line
diff across files the change never touched. Consolidating four repositories does not improve that trade.
The gate is types and tests. The one place hand-formatting has already slipped is
`rust/margin-mail/tests/tsconfig.json`, its siblings' file with every array expanded one element per
line, and the shared config deletes that file outright, which is a better fix than a formatter.

## The Rust side

A cargo workspace needs one filesystem root whose manifest lists every member by path, which means one
git repository. These are four repositories with four release cadences, four version numbers, two
licences, and one of them (Margin Mail) has no remote at all. Each `src-tauri` is its own workspace root
today with its own lockfile (Margin 964 packages, Calendar 565, Docs 962, Mail 668). Nothing short of a
monorepo merge changes that, and [repo-layout.md](repo-layout.md) already made that call. Shared Rust
arrives as git dependencies pinned to a tag, which needs no registry and records a commit hash in
`Cargo.lock`, so the Rust side gets the reproducibility the npm side is missing.

Margin and Margin Docs each carry `stubs/burn-cuda` and `stubs/cubecl-cpu`, empty crates at 0.19.1 and
0.8.1 with matching feature lists, referenced from `[patch.crates-io]` at
`python/margin/src-tauri/Cargo.toml:70-72` and `rust/margin-editor/src-tauri/Cargo.toml:98-100`. They
take the disabled CUDA and LLVM subtrees that `harper-core = "=2.5.0"` version-resolves out of the
graph.

Moving them into the shared repo does not remove the `[patch.crates-io]` block from either app. Cargo
honours `[patch]` only in the top-level manifest of the build; a patch section in a dependency is
ignored. What changes is the source line, from `path = "stubs/burn-cuda"` to
`{ git = "https://github.com/priyanshujain/margin-shared", tag = "v0.3.0" }`, and both `stubs/`
directories disappear. Two caveats follow. The stub version has to keep satisfying what `burn` asks for
and the feature list has to stay a superset of what `burn` references through `burn-cuda?/...`, so the
crate is pinned to the harper version and the two apps move together; that is what the hand sync does
today, except a tag now names the moment. And if either app drops harper, its patch entry becomes an
unused-patch warning rather than an error, so the entry goes when the dependency goes.

`rust/margin-editor/src-tauri/.cargo/config.toml` (`RUST_TEST_THREADS = "1"`, for a suite sharing one
on-disk git repository) is app specific and stays. Both `[profile.dev.package."*"] opt-level = 3` blocks
stay where they are; they are about harper, not about the suite.

## Per app, exactly what changes

**Margin** (`python/margin`) gains the most. A justfile, which is the six-line variable block
(`app := "Margin"`, `binary := "margin-app"`) plus the import, and which is what makes `just install` a
suite-wide rule rather than a rule three apps can follow. A `ci.yml`, which it has never had; today there
is only `appstore.yml` and `release.yml`. The shared tsconfigs, the vite factory, `@types/node` 24, and
`@margin/*` by version in place of `file:./shared`. `pnpm dmg` stays for the App Store path. Four em
dashes in app copy and seven in `website/README.md` clear before `just check` is green.

**Margin Calendar** (`python/margin-caledar`): the tsconfig, vite and justfile changes, and two spec type
errors to fix (`tests/bugs.spec.ts:23`, `tests/touch.spec.ts:42`) before the widened gate passes. It has
no `margin-shared` dependency to migrate, which is why its tokens drifted; picking up `@margin/tokens`
and `@margin/fonts` belongs to that document. Its prose gate is clean today.

**Margin Docs** (`rust/margin-editor`): the CI failure ends when the dependency becomes a published one,
turning a red frontend job green for the first time since it landed. `open "$dest"` comes back with the
shared `_install-macos`. Three spec type errors to fix. Its `build` and `test` blocks are the only users
of the factory's `extra` argument, and `src/markdown/corpus/` goes on the prose gate's skip list.

**Margin Mail** (`rust/margin-mail`) loses the second checkout in `ci.yml:27-31` and in `release.yml`,
and with it the question of why CI clones two repositories. The signing file becomes `apple.env` and
`docs/release.md:25` follows. `scripts/docs-check.mjs` moves into `@margin/config` and the `docs` recipe
comes back from the shared file. It is the closest to the target already: specs type check clean, font
check already in CI, tsconfig already at ES2022.

## What stays per app

The dev server port and the matching `devUrl` in `tauri.conf.json`, because the stagger is what keeps
`reuseExistingServer: true` honest. The inline pre-paint script in each `index.html`: it cannot import,
its storage keys are app prefixed, and it sets different attributes per app, so Margin Docs' per-key form
(which survives a webview that refuses storage) gets copied into the other three by hand. `.gitignore`,
because git has no include mechanism and `core.excludesFile` is per machine.

The CSP block in `tauri.conf.json`, the two capabilities files, and `src-tauri/Cargo.toml` dependencies.
One shared version of any of these would be the union of four permission sets, which is the wrong
direction for a suite that does not reach for spare capabilities.

And Margin Calendar's nix flake until a second app ships Linux through it, Margin's `scripts/` App Store
plumbing, Margin Docs' `.cargo/config.toml`, and Margin Mail's `guide-shots`.
