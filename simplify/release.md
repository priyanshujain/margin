# Release

One release pipeline for four apps, from the measurements in
[.research/ci-release-signing.md](.research/ci-release-signing.md). The channels, the licences, the
reason the updater pubkey lives in an overlay, the contents of `~/.margin-signing` and the Google
Cloud project are settled in [guidelines/distribution.md](guidelines/distribution.md). This document
is what to build so those decisions are enforced in one place instead of copied into four.

## Four copies of one pipeline, and none of them is the good one

The `prepare` job is lines 1 to 81 of `margin-caledar/.github/workflows/release.yml` and lines 1 to
81 of `margin-mail/.github/workflows/release.yml`. `diff` on those 81 lines reports exactly one
change, at line 78: `Margin Calendar $TAG` against `Margin Mail $TAG`. Eighty lines out of
eighty-one are identical, down to the five-attempt rebase loop, the `sleep 3` and the ellipsis in
"main advanced during release".

The `publish` job is worse. `margin-caledar:162-181` against `margin-mail:210-229` differs on one
line, and the difference is punctuation inside an error string: calendar's line 177 still carries an
em dash where mail's line 225 has a comma. Against `margin:199-218` the only substantive difference
in the whole job is the platform key list at line 212, which includes `windows-x86_64`.

Every workflow pins the same eight actions at the same versions: `actions/checkout@v7`,
`actions/setup-node@v6` with `node-version: 26`, `pnpm/action-setup@v6` with `version: 10`,
`dtolnay/rust-toolchain@stable`, `swatinem/rust-cache@v2`, `tauri-apps/tauri-action@v0`,
`cachix/install-nix-action@v31`, `actions/upload-artifact@v4`. Four copies of one pin set, four
places to edit when one of them goes to v8.

The more useful observation is that copying did not converge on a best version. Every capability
worth having exists in exactly one repository, and no repository has more than a third of them.

| Capability | Lives in | Where |
| --- | --- | --- |
| Version string validated before use | Margin Docs | `release.yml:38-41` |
| `[package]`-anchored `Cargo.toml` bump, with a verifying grep | Margin Docs | `release.yml:58-75` |
| Signing distinguishes unconfigured from half configured, fails the second | Margin Docs | `release.yml:158-189` |
| `latest.json` version checked against the tag | Margin Docs | `release.yml:227-231` |
| `latest.json` entries checked for a signature, not just a url | Margin Docs | `release.yml:245-255` |
| The Windows matrix row, and `windows-x86_64` in the gate | Margin | `release.yml:88-102`, `:212` |
| `Cargo.lock` bumped and committed | Margin | `release.yml:47-52`, `:60` |
| `codesign`, `spctl`, `stapler` verification after the build | Margin | `release.yml:188-197` |
| Homebrew tap update | Margin | `release.yml:220-259` |
| The App Store track | Margin | `appstore.yml`, 130 lines |
| The Nix package, pinned and rebuilt | Margin Calendar | `release.yml:187-241`, `ci.yml:74-81` |
| Sibling checkout so the shared package resolves | Margin Mail | `release.yml:101-109`, `ci.yml:19-32` |
| `fonts:check` actually run in CI | Margin Mail | `ci.yml:44` |
| `docs-check.mjs` run in CI | Margin Mail | `ci.yml:51` |

Margin has no `ci.yml` at all. Nothing checks a push or a pull request there, so the first time a
broken tree is noticed is a release build, which is also the moment it is most expensive. Margin
Docs is a milder version of the same thing: it defines `fonts:check` at `package.json:16` and never
calls it from a workflow, so the vendored copy under `public/fonts` can drift from the package
without anything saying so.

## The two real bugs

### Margin Docs cannot install its own dependencies in CI

`margin-editor/package.json:33` declares `"margin-shared": "file:../../python/margin/shared"` and
`pnpm-lock.yaml:1449` records it under that path. `margin-editor/.github/workflows/ci.yml:19` and
`release.yml:119` each do a single `actions/checkout@v7` with no `path:` and no sibling repository.
On a runner the workspace is `/home/runner/work/margin-docs/margin-docs`, so
`../../python/margin/shared` resolves to `/home/runner/work/python/margin/shared`, which does not
exist. `pnpm install --frozen-lockfile` at `ci.yml:29` and `release.yml:141` cannot resolve it.
Margin Docs CI and Margin Docs releases are broken as committed.

Margin Mail has the identical dependency and solved it: two checkouts at `release.yml:101-109`,
itself into `rust/margin-mail` and `priyanshujain/margin` into `python/margin`, plus
`defaults.run.working-directory` at `:97-99`, a rust-cache workspace of
`rust/margin-mail/src-tauri -> target` at `:143`, and `projectPath: rust/margin-mail` on
tauri-action at `:202`. Four coordinated edits, made once, never carried back. That is the concrete
cost of copy-paste in this set: the fix and the bug are in two repos that were the same file.

The shared workflow makes this a one-line input, and [repo-layout.md](repo-layout.md) removes the
underlying relative path entirely by publishing `@margin/*`. Until that lands, the input is the fix.

### Only one app bumps `Cargo.lock`

`Cargo.lock` records the crate's own version. Bumping `Cargo.toml` without it leaves the lock a
release behind, and the next `cargo build` rewrites it under whoever ran it, so the diff lands in an
unrelated commit made by whoever built next.

Margin handles it: an awk pass anchored to `name = "margin-app"` at `release.yml:47-52`, with a
comment saying why, and `src-tauri/Cargo.lock` in the `git add` at `:60`. The other three bump
`tauri.conf.json`, `package.json` and `Cargo.toml` and stop.

Margin Calendar's history has already paid for it twice:

- `3754e4a652dfcfc7dc93507ae58eddf5415e95e4` "Sync the lock file to the version the crate declares"
- `ae5a7b4c0e126f9f034604b907165c6f5582448b` "let cargo.lock catch up with the 0.0.4 bump"

Two manual repair commits for a five-line awk pass that existed in a sibling repo the whole time.
Margin Docs and Margin Mail have the same gap and have not released yet, so theirs is unpaid rather
than absent. All four are consistent right now (0.1.17, 0.0.5, 0.0.1, 0.0.1, lock matching in each);
nothing keeps them that way except that nobody has released since the last repair.

## The reusable workflow

One `workflow_call` workflow in the shared repo, `.github/workflows/release.yml`, with three jobs
and two optional chained workflows. Inputs are only the things the four repos genuinely differ on.

| Input | Type | Default | What it drives |
| --- | --- | --- | --- |
| `app-name` | string | required | The draft release title (`prepare`), and the `.app` path the signing verification checks |
| `platforms` | string | `macos` | Comma list of `macos`, `linux`, `windows`. Drives the build matrix and the `latest.json` key list in `publish` |
| `linux-runner` | string | `ubuntu-22.04` | The Linux matrix row's runner |
| `project-path` | string | `""` | `defaults.run.working-directory`, the rust-cache workspace, and `projectPath` on tauri-action |
| `sibling-repos` | string | `""` | `owner/repo:path` pairs checked out beside the app |
| `needs-google-credentials` | boolean | `false` | Whether the credentials provisioning step runs at all |

`platforms` is the input that matters most, because today the matrix and the publish gate are two
hand-edited lists in every repo and nothing stops them disagreeing. One step expands the list into
both: `macos` gives the `macos-26` universal row and the keys `darwin-aarch64` and `darwin-x86_64`,
`linux` gives the `linux-runner` row and `linux-x86_64`, `windows` gives `windows-latest` and
`windows-x86_64`. A platform that builds and is not checked, or is checked and never built, stops
being expressible.

**`prepare`** runs on `ubuntu-latest`. It resolves the version from the input or bumps the patch out
of `src-tauri/tauri.conf.json`, validates it against `^[0-9]+\.[0-9]+\.[0-9]+$` (Margin Docs'
`release.yml:38-41`, the string becomes a git tag, a TOML value and a JSON value and is typed by
hand), writes all four manifests including `Cargo.lock`, verifies both bumps, commits, tags with the
rebase-and-retry loop, and creates the draft release. Release notes come from `--generate-notes`
rather than `--notes "Release $TAG"`, because tauri-action copies the release body into `latest.json`
and Margin Docs' `src/update.ts:79` already feeds that into a dialog, which today reads "Release
v0.1.18" and nothing else.

**`build`** runs the expanded matrix with `fail-fast: false` and `max-parallel: 1`. The parallelism
limit is required by [guidelines/distribution.md](guidelines/distribution.md) and is in none of the
three matrix workflows today: `margin:91`, `margin-caledar:85` and `margin-mail:85` set `fail-fast`
and stop. Two rows uploading in parallel race on tauri-action's read-modify-write of `latest.json`.
The job does the checkouts (itself at `project-path`, then each `sibling-repos` pair), Linux
packages when the row is Linux, node with `cache: pnpm` (no workflow caches pnpm today, so every job
downloads the whole tree fresh), Rust with the matrix targets, `swatinem/rust-cache@v2` scoped to
the right workspace, `pnpm install --frozen-lockfile`, Google credentials when
`needs-google-credentials`, Apple signing on macOS rows, tauri-action, then the `codesign`, `spctl`
and `stapler` verification from `margin/.github/workflows/release.yml:188-197` with the bundle name
taken from `app-name`.

**`publish`** runs on `ubuntu-latest`, downloads `latest.json`, checks its `.version` against the tag
(Margin Docs `release.yml:227-231`), then checks every expanded platform key for both a url and a
signature (`:245-255`), then flips the draft. An entry with a url and an empty signature is an update
every installed copy will offer, download and reject, so it is not a smaller problem than a missing
entry.

The workflow adds `concurrency: { group: release-${{ github.repository }}, cancel-in-progress:
false }`, which no repo has. Two dispatches at once today would both compute a version from
`tauri.conf.json`, both bump, and race on the push loop at `margin/.github/workflows/release.yml:62-74`.

Homebrew (`margin/release.yml:220-259`) and Nix (`margin-caledar/release.yml:187-241`) become
separate `workflow_call` workflows in the same repo, chained by the caller with `needs: release`.
They are per-app distribution channels with per-app asset names, per-app tap or flake repos and
per-app credentials; a boolean on one job would be worse than two small workflows.

Margin Mail's 229-line `release.yml` becomes this:

```yaml
name: Release

on:
  workflow_dispatch:
    inputs:
      version:
        description: "Release version, e.g. 0.2.0. Leave empty to bump the patch number."
        required: false
        type: string

jobs:
  release:
    uses: priyanshujain/margin-shared/.github/workflows/release.yml@v0.3.0
    with:
      app-name: Margin Mail
      platforms: macos,linux
      project-path: rust/margin-mail
      sibling-repos: priyanshujain/margin:python/margin
      needs-google-credentials: true
    secrets: inherit
```

`secrets: inherit` is what keeps that short: without it every caller redeclares a dozen `APPLE_*`
and `TAURI_SIGNING_*` names. If the shared repo is private, Settings, Actions, Access has to allow
repositories owned by the same account, or the four callers cannot resolve the `uses:` at all.

The same treatment applies to the three `ci.yml` files, which share a `concurrency` block verbatim
(`group: ci-${{ github.ref }}`, `cancel-in-progress: true`) and little else. That second reusable
workflow takes the same checkout inputs plus a `rust-test-command`, because Margin Docs splits its
Rust suite into two steps (`ci.yml:58` and `:78`, the second running `--test-threads=1` against a
suite that shares one folder), and an `extra-frontend-steps` hook for Margin Mail's `fonts:check`
and `docs-check.mjs`. Margin has to call it too, since it has nothing.

## The tauri.conf story

Genuinely per app, and therefore inputs to a generator rather than duplication:

| Value | Margin | Margin Calendar | Margin Docs | Margin Mail |
| --- | --- | --- | --- | --- |
| `productName` | Margin | Margin Calendar | Margin Docs | Margin Mail |
| `identifier` | studio.margin.app | studio.margin.calendar | studio.margin.docs | studio.margin.mail |
| `devUrl` port | 1420 | 1430 | 1440 | 1450 |
| window, min | 1280x820, 920x640 | 1360x900, 880x560 | 1360x900, 880x600 | 1440x900, 880x560 |
| `trafficLightPosition` | absent | 9,25 | absent | 9,25 |
| `bundle.targets` | `"all"` | app, dmg, appimage, deb | app, dmg | app, dmg, appimage, deb |
| CSP additions | `worker-src 'self' blob:` | none | `worker-src`, `asset:` in `img-src` | `frame-src 'self'` |
| `linux.deb.depends` | absent | webkit2gtk-4.1-0, gtk-3-0 | absent | as calendar |
| `macOS.signingIdentity` | absent | absent | absent | `"-"` |
| deep-link plugin | no | yes | no | yes |
| `resources` | dictionaries/en | none | none | none |

Everything else is one fragment written four times: the `$schema`, `category: Productivity`,
`macOS.minimumSystemVersion: 10.15`, the same five icon paths, `titleBarStyle: Overlay`, the build
commands, and the CSP prefix `default-src 'self'; img-src 'self' data: blob:; font-src 'self';
style-src 'self' 'unsafe-inline'; script-src 'self';` with the suffix `connect-src 'self' ipc:
http://ipc.localhost`. All four apps agree on every one of those and none of them can see that they
agree.

Generate the file, do not fragment it. Tauri's `--config` merge only helps at build time and the
committed `tauri.conf.json` still has to be readable by a person opening the repo. A script in
`@margin/config` reads a small per-app descriptor (name, identifier, port, window, targets, CSP
additions, extra plugins) and writes both `src-tauri/tauri.conf.json` and
`src-tauri/tauri.release.conf.json`, with a `--check` mode that compares and exits 1 on any
difference. That shape already exists and works: `margin/shared/bin/sync-fonts.mjs` takes an app
directory and an optional `--check` (`sync-fonts.mjs:11,21,45,62`), is exposed as `fonts:sync` and
`fonts:check`, and runs in CI at `margin-mail/.github/workflows/ci.yml:44`. Copy that interface
exactly rather than inventing a second one.

Generating makes the odd ones out visible. Margin's `bundle.targets: "all"` is a default rather than
a decision, and it is the only reason Margin produces an rpm. Margin Mail's
`macOS.signingIdentity: "-"` is a real decision (an unsigned bundle posts no notifications, see the
`~/.margin-signing` note at `margin-mail/justfile:47`) and has to survive as a per-app value, not get
normalised away.

`tauri.release.conf.json` is four lines of structure and one pubkey in every repo. Generate it the
same way, and put the tauri-apps/tauri#14581 reason in the generator's header, because right now
that reason exists only in `simplify/guidelines/distribution.md:44` and the three sibling
`docs/release.md` files describe the overlay as "where the public half lives" with no explanation.
The next person to tidy a config has nothing telling them not to inline the key. While there, note
that `margin/package.json:11` still has `"dmg": "tauri build --bundles dmg"` with no overlay, which
is exactly the local key-free build the overlay exists to protect.

Two apps carry placeholder pubkeys and have therefore never shipped an update anyone's copy could
verify: `margin-editor/src-tauri/tauri.release.conf.json` says `REPLACE_WITH_TAURI_SIGNER_PUBKEY` and
`margin-mail/src-tauri/tauri.release.conf.json` says `REPLACE_WITH_THE_MINISIGN_PUBLIC_KEY`. Neither
has had a keypair generated, so neither has released at all. Once the shared publish job carries
Margin Docs' signature check, a first release without `TAURI_SIGNING_PRIVATE_KEY` set fails at
publish with a message naming the missing secret rather than shipping a manifest nobody can use,
which is the right failure.

## Signing

Three routes to the same signing step, in three repos, plus a fourth that does not sign.

Margin Calendar has no signing step. `release.yml:152-160` passes `TAURI_SIGNING_PRIVATE_KEY` and its
password to tauri-action and nothing else, so every macOS bundle Calendar has shipped is unsigned and
Gatekeeper refuses it on any machine that has not seen it before. Nobody noticed because nothing
checks.

Margin writes the notarisation `.p8` to `$RUNNER_TEMP` at `release.yml:159-171` and passes the
certificate variables straight into tauri-action's `env` block at `:175-183`. Margin Mail exports
into `$GITHUB_ENV` only when the values are non-empty, using a fixed `MARGIN_EOF` heredoc delimiter,
and warns twice when they are not (`release.yml:167-197`). Margin Docs does the same with a random
delimiter, `EOF_$(openssl rand -hex 12)`, and turns the half-configured case into a hard failure
(`release.yml:158-189`): a certificate with no notarisation credential produces a signed bundle
Gatekeeper still refuses, and doing that quietly is worse than not building. The random delimiter and
the hard failure are both the better version; take Docs' step whole.

The credential itself should be the App Store Connect key, not the Apple ID and app-specific
password that Margin Docs uses at `release.yml:163-165`. One key notarises and uploads to App Store
Connect, so there is a single credential to rotate; it is revocable and reissuable in App Store
Connect without touching a personal account; and an app-specific password is bound to an Apple ID
with 2FA on it, which means the person holding the account is the only one who can reissue it.
Margin's App Store workflow already requires the key (`appstore.yml:114-117`), so the Apple ID route
means the one app shipping on both channels maintains two credentials for the same operation. The
key is already in `~/.margin-signing` as `AuthKey.p8` with `AuthKey.env` beside it. Margin Docs'
`docs/release.md:73-75` tells the reader the opposite of what Margin and Margin Mail's docs tell
them; the workflow and that paragraph both change.

Secret names have to be settled at the same time, because Margin and Margin Mail disagree today.
Margin uses a secret `APPLE_API_KEY_ID` and maps it into the action's `APPLE_API_KEY` environment
variable at `release.yml:183`; Margin Mail has a secret literally named `APPLE_API_KEY`
(`release.yml:176`). Standardise on the Margin spelling, so the four repos hold `APPLE_API_KEY_ID`,
`APPLE_API_ISSUER`, `APPLE_API_KEY_P8`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
`APPLE_SIGNING_IDENTITY` and `APPLE_TEAM_ID`, and the mapping to `APPLE_API_KEY` happens once, inside
the shared workflow.

Only Margin checks the result. `release.yml:188-197` runs `codesign --verify --deep --strict`, then
`spctl --assess --type execute`, then `xcrun stapler validate`, with a comment noting that spctl is
the check a double-clicking user actually meets. That step moves into the shared build job, gated on
the notarisation credential being present, with the hardcoded `Margin.app` replaced by `app-name`.
That alone is what stops Calendar shipping unsigned again.

`margin/scripts/apple-secrets.sh` can serve all four repositories today and does not, because it
lives in one of them. It is already parameterised by `DIR`, `BUNDLE_ID` and `REPO` at `:8-10`, and it
pipes each file straight into `gh secret set` (`:17-32`) so no value is ever echoed into a terminal
or an agent transcript. Move it and `apple-provision.rb` into the shared repo unchanged. One wrinkle
to write down rather than fix silently: `BUNDLE_ID` selects an env file, and only
`studio.margin.app.env` exists, because one Developer ID certificate covers the whole team. That is
why `margin-mail/justfile:47` sources Margin's bundle id, which is right in effect and wrong in
shape. The team-wide file should be the default and `BUNDLE_ID` should only matter where a
provisioning profile does, which is the App Store track.

## Versioning

Four files per app carry the version: `.version` in `src-tauri/tauri.conf.json`, `.version` in
`package.json`, `[package] version` in `src-tauri/Cargo.toml`, and the crate's own entry in
`src-tauri/Cargo.lock`. `tauri.conf.json` is the source of truth because the "leave empty to bump the
patch" path reads it (`release.yml:31` in all four).

Nothing keeps them in sync. The release workflow writes three of them (four in Margin), no `ci.yml`
checks that they agree, and the only reason they agree today is that nobody has hand-edited one.

The single procedure, in the shared `prepare` job: `jq` the two JSON files, awk the `Cargo.toml`
anchored to the `[package]` section, awk the `Cargo.lock` anchored to the crate's own `name`, verify
both with a grep, commit all four in one `chore(release): $TAG`. Then a ten-line check in the shared
CI workflow that reads all four and fails when they disagree, which is the thing that would have
prevented both of Calendar's repair commits.

On the `Cargo.toml` anchoring, be precise rather than dramatic: three repos use
`sed -i "0,/^version = \".*\"/s//.../"`, which takes the first line-anchored `version =` in the file,
and in all four repos today that is line 3 under `[package]`, because every dependency is written in
the inline table form (`tauri = { version = "2", ... }`) and never starts a line with `version`. The
sed is a latent fault, not an active one. It is still worth replacing with Margin Docs' awk
(`release.yml:58-75`), because the day a dependency is written in long form the failure is silent:
the build succeeds and ships the version before.

## Platforms

| Repo | macOS | Linux | Windows | Store |
| --- | --- | --- | --- | --- |
| Margin | universal dmg, signed and notarised, Homebrew cask | deb, rpm, AppImage from `ubuntu-latest` | msi and nsis | Mac App Store pkg |
| Margin Calendar | universal dmg, unsigned | deb and AppImage from `ubuntu-22.04`, plus a Nix flake | none | none |
| Margin Docs | universal dmg, ad hoc signed | none | none | none |
| Margin Mail | universal dmg, ad hoc signed | deb and AppImage from `ubuntu-22.04` | none | none |

Margin builds Linux on `ubuntu-latest`, which contradicts the reasoning Calendar and Mail both
committed to in a comment at `release.yml:88-95` (the bundle will not run against a glibc older than
the one it was linked against, so build on the oldest supported) and produces a bundle with a higher
glibc floor. `ubuntu-22.04` is the default for `linux-runner` and Margin moves to it.

Calendar's Nix flake is the Linux answer the other three should adopt. `flake.nix` is 21 lines, one
input and one system, delegating to `nix/package.nix`, which is 113 lines and repackages the
published `.deb` rather than building from source, for the reason
[guidelines/distribution.md](guidelines/distribution.md) gives. `nix/release.json` is the pin,
`{version, hash}`, currently 0.0.5. The release job downloads the deb, hashes it, writes the pin,
builds the package as proof and pushes the pin to main; `ci.yml:74-81` rebuilds it on every push, so
a nixpkgs change that breaks `autoPatchelfHook` shows up on a pull request rather than at the next
release.

Margin Mail reads `MARGIN_MAIL_PACKAGED_BY` at `src-tauri/src/lib.rs:198` and documents the behaviour
at `docs/release.md:116-119`, and ships no flake, so nothing ever sets that variable and the branch
is dead code. Adopting the flake makes the code it already has mean something. Margin needs the same
plus a `packaged_by` command it does not have. Margin Docs ships no Linux target at all today, so its
`bundle.targets` changes first or not at all.

Mobile readiness is the inverse of the desktop gating, which is worth knowing before anyone plans a
phone build. Margin has a committed iOS Xcode project at `src-tauri/gen/apple` and does **not** gate
the desktop-only plugins: `src-tauri/Cargo.toml:24-25` has `tauri-plugin-process` and
`tauri-plugin-updater` unconditional. Margin Calendar has both `gen/apple` and `gen/android` tracked,
including `MainActivity.kt`, and gates them correctly at `Cargo.toml:43-46`. Margin Docs and Margin
Mail have only `gen/schemas` and gate them too (`Cargo.toml:84-87` and `:134-137`), though Mail's
schemas include `iOS-schema.json` and `mobile-schema.json`. So the one app that could build for a
phone is the one that would fail to, and the three that carry the gate copied the same comment
("There is no auto-updater and no process to restart on a phone: the store is the update channel").

## Per app, in order

**Everyone, first.** Nothing here starts until the trees are committed and pushed.
[risks.md](risks.md) makes that a precondition, and Margin Mail is the hard case: `git remote -v`
returns nothing, so the repo does not exist anywhere but this machine, and both a reusable workflow
reference and Margin Docs' sibling checkout need it to.

**Margin Mail.** Push the repo. Generate the minisign keypair, set `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, replace the placeholder pubkey. Rename the `APPLE_API_KEY`
secret to `APPLE_API_KEY_ID`. Replace `release.yml` and `ci.yml` with calls to the shared workflows,
`platforms: macos,linux`, `project-path: rust/margin-mail`, `sibling-repos:
priyanshujain/margin:python/margin`, `needs-google-credentials: true`. Move `scripts/docs-check.mjs`
into the shared toolchain, since it exists in exactly one repo and
[guidelines/prose-and-docs.md](guidelines/prose-and-docs.md) wants it in all four. Add the flake so
`MARGIN_MAIL_PACKAGED_BY` stops being dead. Mail gains, in one step: version validation, the anchored
Cargo bump, the `Cargo.lock` bump, the manifest and signature checks, the half-configured signing
failure, and the notarisation verification.

**Margin Docs.** Fix the checkout first, before anything else, because CI has never been able to
install: add `sibling-repos` and `project-path`, or wait for `@margin/*` to be published and drop the
relative path instead. Generate the minisign keypair and replace `REPLACE_WITH_TAURI_SIGNER_PUBKEY`.
Switch the signing secrets from `APPLE_ID` and `APPLE_PASSWORD` to the App Store Connect key, and fix
the contradicting paragraph at `docs/release.md:73-75`. Wire `fonts:check` into CI, since
`package.json:16` defines it and nothing calls it. Call the shared workflows with `platforms: macos`.
Docs loses nothing: its version validation, its awk bump and both its manifest checks are the shared
baseline.

**Margin Calendar.** Call the shared workflows with `platforms: macos,linux` and no sibling repos,
since Calendar does not depend on `margin-shared` at all and keeps its own token copies, which is how
the tokens drifted. It gets macOS signing for the first time, and the `spctl` verification means an
unsigned bundle now fails the build instead of shipping. It gets the `Cargo.lock` bump, which
retires the class of `3754e4a` and `ae5a7b4`. The Nix job moves to the shared repo as a chained
`workflow_call` and Calendar keeps calling it with `needs: release`.

**Margin.** It gets a `ci.yml` for the first time, calling the shared CI workflow. Its release
workflow becomes a call with `platforms: macos,linux,windows`. The Linux runner moves from
`ubuntu-latest` to `ubuntu-22.04`, which lowers the glibc floor and is the point of the change.
`bundle.targets: "all"` becomes an explicit list, which means deciding whether the rpm was wanted or
was a side effect; it was a side effect, and dropping it is fine unless somebody says otherwise. The
Homebrew job moves to the shared repo as a chained workflow. `appstore.yml`, `scripts/mas-package.sh`,
`entitlements.mas.plist` and the six Ruby scripts stay in Margin: they are one app's submission, and
templating them for a hypothetical second store app is worse than copying them the day there is one.
Separately, and not part of the release work, gate `tauri-plugin-process` and `tauri-plugin-updater`
in `Cargo.toml:24-25` before anyone builds the committed iOS project.

## One thing that is not broken

`macOS.hardenedRuntime` defaults to `true` in tauri-utils (`config.rs:682` in 2.9.3), so Margin
Calendar and Margin Mail, whose configs omit it, get it anyway. Their bundles are hardened. Margin
and Margin Docs state it explicitly and Calendar and Mail do not, for no reason anyone recorded, and
the generator will settle that difference on its own.

This is written down because it looks exactly like a missing setting in a side-by-side comparison,
and "fixing" it would be a no-op commit in two repos that teaches the next reader the default is
something else. The thing that genuinely differs in that block is Margin Mail's
`macOS.signingIdentity: "-"`, which is deliberate.
