# Distribution

Decided 2026-08-30 for Margin, Margin Calendar and Margin Docs. Margin Mail follows the same shape.

## Channels

All the apps are free. No licence gate, no in-app purchase, so Apple takes no cut and the App Store
anti-steering rules do not apply.

The Mac App Store is the primary macOS channel: sandboxed, no self-updater, a separate build track.
Direct download from margin.73ai.org stays the unrestricted build carrying the Tauri updater. macOS
also ships through the user's own Homebrew tap, `priyanshujain/homebrew-margin`, rather than upstream
homebrew-cask, which the repos do not clear the notability bar for.

Linux ships through a Nix flake. Do not propose the AUR again: the AUR job and PKGBUILD were removed
on 2026-09-03 because the user has no AUR account and signups are restricted. The Nix package is a
binary repackage of the released deb, for the same reason the AUR one was, which is that the Google
OAuth client is embedded at compile time from a file that is not in the repo, so a from-source build
by a stranger produces an app that cannot connect. The wrapper sets a `*_PACKAGED_BY=nix` variable so
the in-app updater announces new versions without trying to install over the store.

Nix is not installed on the user's Mac. Test a flake through the amd64 `nixos/nix` Docker image with
`filter-syscalls = false`, since seccomp fails under emulation, and a named volume on `/nix`.

No migration bridge was built for the library moving into the App Store sandbox container. There are
effectively no existing users and a first-run import is not worth building yet.

## Licensing

Margin is FSL-1.1-MIT: free for anything except a competing product, becoming MIT two years after
each release. Chosen because the user wants open code that nobody else monetises, which is not open
source by the OSI definition. AGPL was ruled out because it is incompatible with the Mac App Store.
Margin Mail is FSL-1.1-MIT too.

Margin Calendar and Margin Docs are still MIT and have not been relicensed. That is an open question,
and consolidating shared code into one package forces it: shared code cannot be under two licences.

## The updater pubkey lives in an overlay

`plugins.updater.pubkey` and `bundle.createUpdaterArtifacts` live in `src-tauri/tauri.release.conf.json`,
a release-only overlay merged with `--config` in the GitHub Actions release workflow. They are
deliberately kept out of the committed `tauri.conf.json`.

The reason is a Tauri bug (tauri-apps/tauri#14581): the mere presence of `plugins.updater.pubkey` in
`tauri.conf.json` makes `tauri build` demand a signing key, which breaks the local key-free build.
The overlay scopes signing and updater artifacts to CI. Only CI-built signed releases need to
self-update anyway.

The release workflow is a manual `workflow_dispatch` with prepare, a build matrix and publish. The
matrix should be `max-parallel: 1`, because tauri-action merges `latest.json` read-modify-write
across platforms and parallel jobs race.

It is not, in any app. Checked on 2026-09-06: `margin/.github/workflows/release.yml:91`,
`margin-caledar/.../release.yml:85` and `margin-mail/.../release.yml:85` set `fail-fast: false` and
nothing else, and Margin Docs has no matrix at all. The constraint was recorded once and then lost,
which is exactly the failure this consolidation is for. The shared build job in
[../release.md](../release.md) sets it.

## Signing

Signing keys and CSRs live in `~/.margin-signing`: Developer ID Application, Apple Distribution, the
installer certificate and the App Store Connect key, each with a `.pass` file. They were generated
with openssl rather than Keychain Access so CI `.p12` files can be rebuilt without a GUI. The
Developer ID is in the login keychain and `codesign` uses it without prompting.

Never print the contents of anything in that directory.

## Google Cloud

One Cloud project, `margin-500217`, numeric id `205537985128`, owned by **pj@73ai.org** and not by
the Google account signed into the apps. Margin Calendar's desktop `google-credentials.json` is a
copy of Margin's, so they share one OAuth desktop client, which means they share the consent screen
and the enabled API list.

Enabling an API is per project, not per client. On 2026-08-10 the calendar scope was granted
correctly and every `calendarList` call still returned 403 `SERVICE_DISABLED`, because the Calendar
API had never been enabled on that project. A `gcloud services enable` run as the gmail account was
denied, because that account does not own the project. If a Google resource comes back empty while
auth succeeds, check the API is enabled before suspecting sync, and run the enable as pj@73ai.org.

Phones share the desktop client deliberately. A Desktop client may redirect to loopback on any port
without registering it, and Google's token endpoint checks the client id, secret and redirect rather
than the calling OS. Verified on 2026-08-12 on both an iOS simulator and an Android emulator. This
works because the protocol does not check, not because Google blesses it; the fallback if they ever
enforce it is a per-platform client, which the code already supports.

One thing still argues for a real iOS client: it switches iOS to `ASWebAuthenticationSession`, which
shares Safari's session, so the user is not asked to sign in again. Android needs nothing, because
Chrome Custom Tabs already share Chrome's cookies, which was measured rather than assumed. The iOS
session sharing could not be confirmed on the simulator and needs a real device.

The client secret is not confidential for an installed app. Embed it, through the same release
overlay pattern as the updater pubkey, so local builds stay clean.

Refresh tokens are not in any keychain. They are XChaCha20-Poly1305 sealed in a file under the app
data directory, so the old `security find-generic-password` check no longer applies anywhere.

## Never touch cloud infrastructure unasked

Reading is fine: list, describe, get, dry runs. Changing is not. Do not create, delete, rename or
reconfigure a project, bucket, database, service, key, credential, IAM binding or DNS record, do not
enable or disable an API, and do not attach billing without being asked for that exact thing on that
exact resource. Permission does not carry forward to the next request.

If something turns out to be blocked, say it is blocked and why. Do not route around it. A workaround
that provisions new infrastructure is a much bigger decision than the one that was made.
