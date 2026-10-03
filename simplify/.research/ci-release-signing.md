# CI, packaging, signing, the updater, distribution

Four repos, four hand-maintained copies of the same release pipeline. This is what is in them, what
has already drifted, and what a shared version would have to keep per app. Paths: `margin` is
`/Users/pj/Workspace/projects/python/margin`, `margin-calendar` is
`/Users/pj/Workspace/projects/python/margin-caledar` (misspelled on disk), `margin-docs` is
`/Users/pj/Workspace/projects/rust/margin-editor`, `margin-mail` is
`/Users/pj/Workspace/projects/rust/margin-mail`.

## The workflows

| repo | release.yml | ci.yml | other |
| --- | --- | --- | --- |
| margin | 259 lines | none | appstore.yml, 130 lines |
| margin-calendar | 241 | 81 | |
| margin-docs | 258 | 78 | |
| margin-mail | 229 | 92 | |

margin has no CI workflow at all. Nothing checks a push or a pull request there; the first time a
broken tree is noticed is a release build.

### How near-duplicate the release YAML is

Whole-file changed lines (`diff | grep -c '^[<>]'`) run from 126 (calendar vs mail) through 136
(margin vs calendar), 144 (margin vs mail), 211 (docs vs mail), 225 (calendar vs docs) to 241
(margin vs docs). Those numbers overstate the difference, because they count reflowed comment
blocks. The structural duplication is much worse. The whole `prepare` job, lines 1 to 81 of both
files, is byte-identical between margin-calendar and margin-mail except for one line: the title at
`margin-caledar/.github/workflows/release.yml:78` says `Margin Calendar $TAG` and
`margin-mail/.github/workflows/release.yml:78` says `Margin Mail $TAG`. Nothing else in 81 lines
differs.

The `publish` job is the same story. `margin-caledar/.github/workflows/release.yml:162-181` against
`margin-mail/.github/workflows/release.yml:210-229` differs on one line, and that line is
punctuation inside an error string (calendar still has an em dash at line 177, mail rewrote it as a
comma). Against margin, `release.yml:199-218`, the only real difference is the platform key list:
margin checks `darwin-aarch64 darwin-x86_64 linux-x86_64 windows-x86_64` at line 212, the other two
check the same list without Windows.

Every workflow pins the same actions at the same versions: `actions/checkout@v7`,
`actions/setup-node@v6` with `node-version: 26`, `pnpm/action-setup@v6` with `version: 10`,
`dtolnay/rust-toolchain@stable`, `swatinem/rust-cache@v2`, `tauri-apps/tauri-action@v0`,
`cachix/install-nix-action@v31`, `actions/upload-artifact@v4`. Four copies of one pin set.

### Triggers, permissions, concurrency, caching

All four releases are `workflow_dispatch` only, with one optional `version` string input, and
`permissions: contents: write` at workflow level. margin's `appstore.yml:16-17` is the one workflow
with `contents: read`.

No release workflow has a `concurrency` block. Two dispatches at once would both compute a version
from `tauri.conf.json`, both bump, and race on the push-with-rebase loop at
`margin/.github/workflows/release.yml:62-74`. The three `ci.yml` files do have one, identical in all
three (`group: ci-${{ github.ref }}`, `cancel-in-progress: true`): another three-way copy.

Cargo is cached everywhere through `swatinem/rust-cache@v2`. pnpm is cached nowhere: no workflow
sets `cache: pnpm` on `setup-node`, so every job downloads the whole tree fresh.

### Matrix and runners

margin, `release.yml:88-102`: three rows, `macos-26` universal, `ubuntu-latest`, `windows-latest`.
Only margin builds Windows, and only margin installs `rpm` (`release.yml:124`). margin-calendar and
margin-mail, both `release.yml:88-95`: two rows, `macos-26` universal and `ubuntu-22.04`, both
carrying the same comment about 22.04 being the glibc baseline. margin builds Linux on
`ubuntu-latest` instead, contradicting the reasoning the other two committed to and producing a
bundle with a higher glibc floor. margin-docs, `release.yml:111-117`: no matrix, one `macos-26`
runner, with a good comment on why a matrix of one is where a stale Linux row survives.

margin-mail is the only one that needs two checkouts, `release.yml:101-109`: itself into
`rust/margin-mail` and `priyanshujain/margin` into `python/margin`, because `package.json` has
`"margin-shared": "file:../../python/margin/shared"`. That forces `defaults.run.working-directory`
(`release.yml:97-99`), a different rust-cache workspace path (`release.yml:143`), and
`projectPath: rust/margin-mail` on the tauri-action (`release.yml:202`).

**margin-docs has the same relative dependency and does not do this.**
`margin-editor/package.json` declares `margin-shared` at `file:../../python/margin/shared` and
`pnpm-lock.yaml:1449` records it under that path, but `margin-editor/.github/workflows/ci.yml:19`
and `release.yml:119` each do a single checkout with no sibling repo. `pnpm install
--frozen-lockfile` cannot resolve that path. margin-docs CI and margin-docs releases are broken as
committed. That is the single most concrete cost of copy-paste here: the fix landed in mail and was
never carried back.

### Which workflow is most evolved

margin-docs, and it is not close. It is the only one that validates the version string before using
it (`release.yml:38-41`), the only one that bumps `Cargo.toml` with a `[package]`-anchored awk pass
and then verifies the result (`release.yml:58-75`), the only one whose Apple signing step
distinguishes "unconfigured" from "half configured" and fails the second case
(`release.yml:176-189`), the only one that checks the manifest version against the tag
(`release.yml:227-231`), and the only one that checks `latest.json` carries a signature and not just
a url (`release.yml:245-255`). Its comments explain why each check exists.

margin is the most complete in scope: the only Windows row, the only App Store workflow, the only
post-build `codesign`/`spctl`/`stapler` verification (`release.yml:188-197`), the only Homebrew tap
job (`release.yml:220-259`), and the only bump step that rewrites `Cargo.lock` (`release.yml:47-52`).
margin-calendar is the only one with a Nix job (`release.yml:187-241`). margin-mail is the plainest:
the two-repo checkout and nothing else the others lack. Nobody has all of it, and every good idea
lives in exactly one repo.

## tauri.conf.json side by side

| | margin | margin-calendar | margin-docs | margin-mail |
| --- | --- | --- | --- | --- |
| productName | Margin | Margin Calendar | Margin Docs | Margin Mail |
| version | 0.1.17 | 0.0.5 | 0.0.1 | 0.0.1 |
| identifier | studio.margin.app | studio.margin.calendar | studio.margin.docs | studio.margin.mail |
| devUrl port | 1420 | 1430 | 1440 | 1450 |
| window | 1280x820, min 920x640 | 1360x900, min 880x560 | 1360x900, min 880x600 | 1440x900, min 880x560 |
| titleBarStyle | Overlay | Overlay | Overlay | Overlay |
| trafficLightPosition | absent | 9,25 | absent | 9,25 |
| bundle.targets | `"all"` | app, dmg, appimage, deb | app, dmg | app, dmg, appimage, deb |
| category | Productivity | Productivity | Productivity | Productivity |
| macOS.minimumSystemVersion | 10.15 | 10.15 | 10.15 | 10.15 |
| macOS.hardenedRuntime | true | absent | true | absent |
| macOS.signingIdentity | absent | absent | absent | `"-"` |
| linux.deb.depends | absent | webkit2gtk-4.1-0, gtk-3-0 | absent | same as calendar |
| deep-link plugin | no | yes | no | yes |
| resources | dictionaries/en | none | none | none |
| copyright | present | absent | absent | absent |

Line references: `margin/src-tauri/tauri.conf.json:3-5,29,40-47`,
`margin-caledar/src-tauri/tauri.conf.json:3-5,49-54,65-75`,
`margin-editor/src-tauri/tauri.conf.json:3-5,29-32,43-46`,
`margin-mail/src-tauri/tauri.conf.json:3-5,45,56-64`.

`hardenedRuntime` defaults to `true` in tauri-utils
(`~/.cargo/registry/src/index.crates.io-.../tauri-utils-2.9.3/src/config.rs:682`), so the two that
omit it get it anyway. Two repos state it and two do not, for no reason.

The CSP is four variations on one string. All four begin `default-src 'self'; img-src 'self' data:
blob:; font-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self';` and end with
`connect-src 'self' ipc: http://ipc.localhost`. margin adds `worker-src 'self' blob:`, margin-docs
adds that plus `asset: http://asset.localhost` to `img-src`, margin-mail adds `frame-src 'self'`,
margin-calendar adds nothing. Every app declares the same five icon paths.

`bundle.targets: "all"` in margin is why margin gets an rpm and the others do not. It is a default
rather than a decision.

## Signing and notarisation

Local credentials live in `~/.margin-signing`, overridable with `MARGIN_SIGNING_DIR`. Only two
repos reference it. In margin: `scripts/apple-provision.rb:39`, `scripts/apple-secrets.sh:8`,
`scripts/mas-upload-local.sh:15`, documented at `docs/publishing.md:213-226`. In margin-mail:
`justfile:47` and `docs/release.md:25-26,40`.

The directory holds, by name: three `.p12` files (`developer-id.p12`, `apple-distribution.p12`,
`mac-installer.p12`) each with a sibling `.pass` file, a `.provisionprofile` named after the bundle
id, `AuthKey.p8` with `AuthKey.env` beside it holding the key id and issuer, and an env file per
bundle id (`studio.margin.app.env`) exporting `APPLE_TEAM_ID`, `APPLE_SIGNING_IDENTITY`,
`MAS_APP_IDENTITY` and `MAS_INSTALLER_IDENTITY`. Private keys are generated locally so Apple only
ever sees a CSR, and each `.p12` bundles Apple's intermediate so a fresh CI keychain can build a
chain (`docs/publishing.md:214-218`).

`margin/scripts/apple-secrets.sh` pushes all of it into a repository's Actions secrets by piping
files straight into `gh secret set` so nothing is echoed (`apple-secrets.sh:17-32`). It is already
parameterised: `DIR`, `BUNDLE_ID` and `REPO` are env-overridable (`apple-secrets.sh:8-10`). It could
serve all four repos today and does not, because it lives in one of them. margin-mail's local build
sources `$MARGIN_SIGNING_DIR/studio.margin.app.env`, hardcoded to margin's bundle id
(`justfile:47`), which is right in effect (one certificate covers the team) and wrong in shape.

The three release workflows arrive at the same signing step by three different routes.
margin-calendar has none at all: `release.yml:152-160` passes only the Tauri updater key, so
calendar ships unsigned macOS bundles. margin writes only the notarisation `.p8` to disk
(`release.yml:159-171`) and passes the certificate variables directly to the action
(`release.yml:175-183`). margin-mail exports certificate variables into `$GITHUB_ENV` only when they
are non-empty, with a heredoc delimiter, and warns twice when they are not
(`release.yml:167-197`). margin-docs does the same with a random heredoc delimiter and a hard
failure on the half-configured case (`release.yml:158-197`).

The secret names have drifted. margin uses `APPLE_API_KEY_ID` as the secret and maps it to
`APPLE_API_KEY` in the action environment (`release.yml:183`). margin-mail uses a secret literally
called `APPLE_API_KEY` (`release.yml:176`). margin-docs uses the Apple ID and app-specific password
route instead: `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` (`release.yml:163-165`), which is the
one `docs/publishing.md:22-24` explicitly argued against, because the App Store Connect key does
notarisation and store upload with one credential to rotate.

Only margin verifies the result. `release.yml:188-197` runs `codesign --verify --deep --strict`,
then `spctl --assess --type execute`, then `xcrun stapler validate`, with a comment that spctl is
the check a double-clicking user actually meets. No other repo checks that its signed bundle is
notarised.

### The App Store track

`margin/appstore/` is listing content, not code: one text file per App Store Connect field under
`appstore/metadata/en-US/` (name, subtitle, description, keywords, promotional_text,
release_notes, privacy_url, support_url, marketing_url, beta_description), review contact details
under `appstore/metadata/`, and five 2560x1600 frames under `appstore/screenshots/` that are a CSS
rebuild of the app rather than a screen capture. `margin/target-mas/` is a build output directory,
holding `Margin.pkg`; `appstore.yml:125-129` uploads `target-mas/*.pkg` as an artifact.

`scripts/mas-package.sh` covers the distance Tauri does not: it stamps `CFBundleVersion` from the
workflow run number (`mas-package.sh:27-30`), copies the provisioning profile into the bundle before
signing, widens permissions so Apple can read every file (`mas-package.sh:33-39`), substitutes
`__TEAM_ID__` into `entitlements.mas.plist`, signs nested code first and never with `--deep`, then
`productbuild`s the result (`mas-package.sh:44-58`). `entitlements.mas.plist` declares four
entitlements, each justified in a comment: app-sandbox, network.client, network.server for the
loopback OAuth listener, and files.user-selected.read-write. `src-tauri/Info.plist:8-9` declares
`ITSAppUsesNonExemptEncryption` false. Six Ruby scripts drive the Developer Portal and App Store
Connect through fastlane's spaceship.

None of this exists in the other three, and calendar and mail both have a Google OAuth loopback
listener, so if either goes to the store it needs the same entitlement and the same review note.

## The updater

One endpoint per app, all on GitHub releases:

| repo | endpoint | pubkey |
| --- | --- | --- |
| margin | `.../priyanshujain/margin/releases/latest/download/latest.json` | real |
| margin-calendar | `.../margin-calendar/releases/latest/download/latest.json` | real |
| margin-docs | `.../margin-docs/releases/latest/download/latest.json` | `REPLACE_WITH_TAURI_SIGNER_PUBKEY` |
| margin-mail | `.../margin-mail/releases/latest/download/latest.json` | `REPLACE_WITH_THE_MINISIGN_PUBLIC_KEY` |

All at line 8 to 11 of each `src-tauri/tauri.release.conf.json`. Two of the four have never had a
keypair generated, so neither has released.

**All four already use the overlay workaround.** No `tauri.conf.json` carries
`plugins.updater.pubkey`; all four keep it plus `bundle.createUpdaterArtifacts: true` in
`tauri.release.conf.json`, merged with `--config src-tauri/tauri.release.conf.json` in the build
args. What did not propagate is the reason. Only margin records it, and only outside `docs/`:
`simplify/guidelines/distribution.md:44` and `simplify/.research/memories-raw.md:283` name
tauri-apps/tauri#14581, that the mere presence of the pubkey makes `tauri build` demand a signing
key and would break the key-free local build (`margin/package.json` still has `"dmg": "tauri build
--bundles dmg"` with no overlay). The three sibling `docs/release.md` files describe the overlay as
"where the public half lives" and give no reason, so the next person to tidy a config has nothing
telling them not to inline it.

The overlay carries a second job nobody has written down: it is the flag that switches the plugin
on. Every app registers the plugin conditionally on the merged config, ported verbatim four times:
`margin/src-tauri/src/lib.rs:159-161`, `margin-caledar/src-tauri/src/lib.rs:260-262`,
`margin-editor/src-tauri/src/lib.rs:263-265`, `margin-mail/src-tauri/src/lib.rs:265-267`. Two of the
comments say "Ported from margin's lib.rs" outright.

`margin/src-tauri/src/updates.rs` is the only per-channel logic anywhere. `channel()` at lines 17 to
26 reads which plugin key the merged config declares, `updater` meaning direct download and
`appstore` meaning store, and a `_MASReceipt` in the bundle overrides both (lines 28 to 37), so a
store build cannot self-update even if built with the updater in it. `appstore_latest()` at lines 51
to 84 asks `itunes.apple.com/lookup` with a cache-busting timestamp. No sibling has or needs this.

Release notes are surfaced but empty. All four create the draft with `--notes "Release $TAG"`
(`release.yml:78` in calendar, docs and mail; `:84` in margin), tauri-action copies the release body
into `latest.json`, so `update.body` is the literal string "Release v0.1.18".
`margin-editor/src/update.ts:79` passes that into `useUpdate.offer(version, notes)`, which
`store/useUpdate.ts:39` calls "the release notes, as the release wrote them".

The four update UIs are four different things: margin has `src/updater.ts` (111 lines) plus an
`UpdateDialog.tsx` and a store; margin-docs has the most developed, `src/update.ts` (171 lines) with
a daily background check, a 6 second launch delay and explicit handling of "this build has no
updater in it" (`update.ts:39-56`); margin-calendar has a 41 line toast-only version
(`src/keys/updates.ts`) whose header says it is margin's minus the dialog; margin-mail has no
updater module, just an inline `checkForUpdates` in `App.tsx:98-118` and a panel in
`screens/Settings.tsx:2465-2540`.

`packaged_by()` is the Nix escape hatch, ported twice: `margin-caledar/src-tauri/src/lib.rs:240-245`
reading `MARGIN_CALENDAR_PACKAGED_BY`, `margin-mail/src-tauri/src/lib.rs:197-201` reading
`MARGIN_MAIL_PACKAGED_BY`. mail reads a variable nothing sets, because mail has no Nix package.

## Versioning

Three files per app, all bumped by the release workflow and by nothing else: `.version` in
`src-tauri/tauri.conf.json`, `.version` in `package.json`, and `[package] version` in
`src-tauri/Cargo.toml`. `tauri.conf.json` is the source of truth, since the "leave empty to bump the
patch" path reads it (`release.yml:31` in all four). All four are consistent right now: margin
0.1.17, calendar 0.0.5, docs 0.0.1, mail 0.0.1, with `Cargo.lock` matching in each.

Nothing enforces it. There is no check in any `ci.yml` that the three agree, so the only thing
keeping them together is that a human never edits one by hand.

The `Cargo.lock` problem is history rather than theory. Only margin bumps the lock, with an awk pass
and a comment explaining that the lock records the crate's own version
(`margin/.github/workflows/release.yml:47-52`), and it is the only repo that adds `Cargo.lock` to
the release commit (`:60`). margin-calendar does not, and its history carries two manual repair
commits for exactly this: `3754e4a Sync the lock file to the version the crate declares` and
`ae5a7b4 let cargo.lock catch up with the 0.0.4 bump`. docs and mail have the same gap and have not
released yet.

The bump itself is three different implementations. margin and margin-mail and margin-calendar use
`sed -i "0,/^version = \".*\"/s//.../"` on `Cargo.toml`, which takes the first `version =` line in
the file. margin-docs replaced it with a `[package]`-anchored awk pass plus a verification grep
(`release.yml:58-75`), with a comment explaining that a long-form dependency puts `version = "0.4"`
on its own line and bumping that one ships the version before. margin-mail's `Cargo.toml` is 6568
bytes with many long-form dependencies, so it is the repo most exposed to the bug and it has the old
code.

## Linux, Windows, mobile

What each app actually ships:

| repo | macOS | Linux | Windows | store |
| --- | --- | --- | --- | --- |
| margin | universal dmg, signed and notarised, Homebrew cask | deb, rpm, AppImage from ubuntu-latest | msi and nsis from windows-latest | Mac App Store pkg |
| margin-calendar | universal dmg, unsigned | deb and AppImage from ubuntu-22.04, plus a Nix flake | none | none |
| margin-docs | universal dmg, ad hoc signed today | none | none | none |
| margin-mail | universal dmg, ad hoc signed today | deb and AppImage from ubuntu-22.04 | none | none |

margin-calendar's `flake.nix` is 21 lines: one input, one system (`x86_64-linux`), an overlay and a
package, both calling `nix/package.nix`. That file is 113 lines and repackages the published `.deb`
rather than building from source, justified at lines 1 to 4 by the OAuth client being embedded at
compile time from a file that is not in the repo. `autoPatchelfHook` relinks it against nixpkgs'
webkit2gtk so it runs as a native Wayland client instead of the AppImage's Xwayland fallback, a
generated launcher points libglvnd at nixpkgs' Mesa when `/run/opengl-driver` is absent
(`package.nix:68-94`), and `preFixup` sets `MARGIN_CALENDAR_PACKAGED_BY=nix` (`package.nix:98-103`).

`nix/release.json` is the pin, `{version, hash}`, currently 0.0.5. The `nix` job
(`release.yml:187-241`) runs after publish, downloads the deb, hashes it, writes the pin, builds the
package as proof, and pushes the pin to main with the same rebase loop as the version bump.
`ci.yml:74-81` rebuilds it on every push. This replaced an AUR package, rationale at
`docs/release.md:41-81`; no AUR file is left in the tree.

margin-mail reads `MARGIN_MAIL_PACKAGED_BY` and documents the Nix behaviour at `docs/release.md:116-119`
but ships no flake, so that path is dead code today.

Mobile is scaffolded in two repos. `margin/src-tauri/gen/apple` is a committed iOS Xcode project;
`margin-caledar/src-tauri/gen/` has both `apple` and `android` tracked, including
`app/src/main/java/studio/margin/calendar/MainActivity.kt`. margin-docs and margin-mail have only
`gen/schemas`, though margin-mail's include `iOS-schema.json` and `mobile-schema.json`.

The desktop-only cfg gating is the same three lines in three repos, with the same comment ("There is
no auto-updater and no process to restart on a phone: the store is the update channel"):
`margin-caledar/src-tauri/Cargo.toml:43-46`, `margin-editor/src-tauri/Cargo.toml:84-87`,
`margin-mail/src-tauri/Cargo.toml:134-137`, each gating `tauri-plugin-process` and
`tauri-plugin-updater` behind `cfg(not(any(target_os = "android", target_os = "ios")))`. margin, the
repo that actually has a committed iOS project, does not gate them:
`margin/src-tauri/Cargo.toml:24-25` has both unconditional. `capabilities/desktop.json` is in all
four with the same two permissions, `updater:default` and `process:allow-restart`.

`src-tauri/build.rs` is two files across four repos: margin and margin-docs share one (39 bytes),
margin-calendar and margin-mail share the credential-embedding one (1171 bytes), byte-identical.

## docs/release.md

margin-calendar 105 lines, margin-docs 117, margin-mail 119. margin has none; its equivalent is
`docs/publishing.md`, 229 lines, covering three distribution channels the others do not have.

The "Installing locally" opening is near-identical in all three, down to "It is the same command
whether or not the app is already installed, so it doubles as the update" and the sentence about a
bundle going half old and half new. "Cutting a release" is the same paragraph in all three with the
app's own manifest list. "Windows is not built" appears in all three, calendar and mail sharing the
identical follow-up about a runner, `msi`/`nsis`, and the gate then wanting `windows-x86_64`.

Where they genuinely diverge: calendar has a 41 line Linux and Nix section nobody else has; docs has
a long honest section on self-update being impossible until a Developer ID certificate exists
(`docs/release.md:105-117`) and a paragraph on why the bundle asks for the hardened runtime and no
entitlements; mail has a Signing section built around `~/.margin-signing` with a `gh secret set`
recipe (`docs/release.md:37-48`) and a "Before the first release" section covering both the missing
updater key and Google restricted scope verification.

The signing advice contradicts itself across the set. docs tells the reader to use `APPLE_ID` and an
app-specific password (`docs/release.md:73-75`), mail and margin tell them to use an App Store
Connect key. Both cannot be the house rule.

## margin/website

Astro 5, one page, deployed by hand: `package.json` has `"deploy": "astro build && npx wrangler
pages deploy dist --project-name=margin --commit-dirty=true"`. No workflow deploys it.

Downloads resolve at build time, not at request time. `website/src/data/release.ts:29-45` fetches
`api.github.com/repos/priyanshujain/margin/releases/latest`, picks one asset per platform by
filename suffix from `site.ts:40-43` (`.dmg`; `.exe` then `.msi`; `.deb` then `.AppImage` then
`.rpm`), and falls back to the releases page on any error including the 8 second timeout. Astro runs
that once at build, so the buttons point at whatever was latest when the site was last deployed and
a release not followed by a deploy leaves stale links. `site.ts:23` carries the repo slug, so the
whole thing is one constant away from serving a sibling app.

## What to build

**One reusable workflow, `workflow_call`, in a shared repo.** The publish job goes in verbatim, the
prepare job goes in with the title as an input, and the build job goes in with the matrix as an
input. Inputs the four repos actually differ on, and nothing else:

- `app-name`: release title and, on margin, the dmg filename in the Homebrew job.
- `platforms`: a list like `macos,linux,windows`, driving both the build matrix and the
  `latest.json` key list in publish. Those two must not be able to disagree; today they are two
  hand-edited lists in every repo.
- `linux-runner`: default `ubuntu-22.04`, since three repos already agree that is the right glibc
  baseline and margin's `ubuntu-latest` is an oversight.
- `project-path` and `sibling-repos`: empty for margin and calendar, `rust/margin-mail` plus
  `priyanshujain/margin` for mail, and the same for docs, which needs it and does not have it.
- `needs-google-credentials`: boolean. margin, calendar and mail true; docs false.
- `post-publish`: which trailing job runs, `homebrew` for margin, `nix` for calendar, none for the
  others. These are different enough that they should be separate reusable workflows the caller
  chains, not a flag.

Take margin-docs' version validation, its `[package]`-anchored Cargo bump, its manifest-version and
signature checks, and its half-configured signing failure as the baseline; add margin's `Cargo.lock`
bump and its `codesign`/`spctl`/`stapler` verification. That combination exists in no repo today.
Add `concurrency: group: release-${{ github.repository }}, cancel-in-progress: false` and `cache:
pnpm` on `setup-node`, neither of which exists anywhere.

The three `ci.yml` files should share a second reusable workflow with the same inputs plus a
`rust-test-command` and an `extra-frontend-steps` hook, since margin-docs splits its Rust suite
(`ci.yml:58,78`) and margin-mail adds docs and fonts checks (`ci.yml:44,51`). margin must call it.

**A shared tauri.conf fragment.** Generate rather than fragment: Tauri's `--config` merge only helps
at build time and the committed file still has to be readable. A small script in the shared package
that takes app name, identifier, dev port, window size, targets and any extra CSP directives, and
writes `tauri.conf.json`, with a `--check` mode wired into CI the way `pnpm fonts:check` already is.
That kills four copies of the icon list, the category, the min system version, the base CSP and the
window defaults, and it makes the odd ones out visible: margin's `targets: "all"` and the two
repos that state `hardenedRuntime` redundantly.

`tauri.release.conf.json` is four lines of structure and one pubkey. Generate it the same way, and
put the tauri#14581 reason in the generator's header so it survives the next cleanup.

**One signing procedure, documented once.** `margin/docs/publishing.md:193-229` plus
`margin-mail/docs/release.md:18-48` is already the whole thing; it needs to be one page in the shared
repo covering `~/.margin-signing`'s layout, the three certificate types and why they cannot be
combined, and the App Store Connect key as the single notarisation credential. Settle the
`APPLE_ID` versus API key question in favour of the key, and fix margin-docs' workflow to match.
Move `scripts/apple-secrets.sh` and `apple-provision.rb` into the shared repo unchanged: they are
already parameterised by `DIR`, `BUNDLE_ID` and `REPO`. Standardise on `APPLE_API_KEY_ID` as the
secret name in all four, since margin and mail currently disagree. Add the `spctl` and `stapler`
verification to the shared build job so margin-calendar stops shipping unsigned macOS bundles
without anyone noticing.

**One versioning convention.** `tauri.conf.json` is the source, the workflow writes `package.json`,
`Cargo.toml` and `Cargo.lock`, and a CI check asserts all four agree. Roughly ten lines, and it
would have prevented both of margin-calendar's manual repair commits. While there, replace
`--notes "Release $TAG"` with `--generate-notes` or an extracted `CHANGELOG.md` section, since it
feeds a dialog margin-docs already built.

## What genuinely cannot be shared

The identifiers, product names, dev ports, window sizes and the CSP additions each app needs;
those are inputs, not duplication.

margin's App Store track. The listing text, the screenshots, the entitlements and the six Ruby
scripts are about one app's submission. `mas-package.sh` and `entitlements.mas.plist` could be
templated later if a second app goes to the store, but there is no second app and templating for a
hypothetical one is worse than copying it when the day comes.

margin's Homebrew job and margin-calendar's Nix job. Both are per-app distribution channels with
per-app asset names, per-app tap or flake repos, and a per-app credential. They can be reusable
workflows the caller chains, but they are not one job with a flag.

`margin/website`. It is one product's marketing site, with pricing, an offer counter and store
logos. `data/release.ts` is genuinely generic and could move to the shared package if a second app
ever gets a site.

The updater UIs. Four apps have four different answers to what should happen when an update is
found, and margin's App Store channel logic in `updates.rs` has no meaning in the other three. The
plugin registration block, the `packaged_by` command and the `capabilities/desktop.json` permissions
are the same three things four times over and are worth sharing; the dialogs are not.

The per-app minisign keypair. One key per app is correct: the pubkey is baked into every shipped
binary and cannot be rotated without stranding installed copies, so a shared key would make one
compromise a four-app problem.
