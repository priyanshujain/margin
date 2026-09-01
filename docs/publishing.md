# Publishing

Margin goes out through three doors, and they are not the same app.

The **direct download** from [margin.73ai.org](https://margin.73ai.org) is the full one. It is not
sandboxed, it updates itself, and nobody stands between it and the person using it. The **Homebrew
cask** is the same file with a command instead of a browser. The **Mac App Store** copy is a
sandboxed build with the updater taken out, because Apple requires both. It exists because most
people will never download a `.dmg` from GitHub, and telling them to is how you end up with no
users rather than principled ones.

## Signing and notarization

Every macOS bundle is signed with a Developer ID Application certificate and notarized by Apple
before it is published. This is not optional any more: an unsigned bundle on a current macOS opens
to "Apple could not verify this app is free of malware" with no obvious way past it, and the way
past it that does exist teaches people to click through exactly the warning that is worth reading.

The release workflow does it. `tauri-action` picks up `APPLE_CERTIFICATE` and
`APPLE_CERTIFICATE_PASSWORD` (a base64 `.p12` and its password), signs with
`APPLE_SIGNING_IDENTITY`, and notarizes with an App Store Connect API key: `APPLE_API_ISSUER`,
`APPLE_API_KEY_ID`, and `APPLE_API_KEY_P8`, the last being the base64 of the `.p8` file. The key is
used rather than an Apple ID and app-specific password because the same key does the App Store
upload, so there is one credential to rotate instead of two.

The step after the build is the one that matters. `codesign --verify` only says the signature is
internally consistent; `spctl --assess` is what a person double-clicking the file actually meets,
and it does not pass until `stapler` has attached the notarization ticket to the bundle. If that
step goes green the download works on a machine that has never heard of Margin. If it is skipped
because no key was configured, the workflow says so in a warning rather than shipping something
that looks fine and is not.

## Homebrew

```
brew tap priyanshujain/margin
brew trust priyanshujain/margin
brew install --cask margin
```

Homebrew 6 refuses to load a cask from a tap it has not been told to trust, and the error it raises
instead says nothing about installing, so the trust line belongs in every set of instructions
rather than being left for people to discover.

The tap is [priyanshujain/homebrew-margin](https://github.com/priyanshujain/homebrew-margin) rather
than upstream `homebrew-cask`, which has a notability bar Margin does not clear yet. What that
costs is the two setup lines above, and the tap keeps working as a fallback if the cask ever does
go upstream.

The `homebrew` job at the end of the release workflow downloads the `.dmg` that was just published,
takes its sha256, and rewrites the version and hash in `Casks/margin.rb`. It runs after the publish
gate, so the cask can never point at a release that is still a draft. Without
`HOMEBREW_TAP_DEPLOY_KEY` it warns and does nothing, which leaves the cask on the previous version
rather than failing a release that otherwise succeeded.

That secret is an SSH deploy key registered on the tap, not a personal access token. A token would
carry the whole account; the deploy key reaches the tap and nothing else, so a leak from a release
job cannot touch the app repositories.

## The Mac App Store

Tauri has no App Store target, so `scripts/mas-package.sh` covers the distance between the `.app`
and something App Store Connect will take. It embeds the provisioning profile, resolves the team
identifier into the entitlements, signs, and wraps the result with `productbuild`.

The order is load-bearing. The profile goes in before `codesign` runs, because the signature covers
it. Tauri's own signing is switched off in this build for the same reason: it would sign a bundle
with no profile in it. Nested code, if there ever is any, is signed before the bundle that contains
it, and never with `--deep`, which would apply the app's entitlements to everything inside.

`CFBundleVersion` comes from the workflow run number. App Store Connect refuses an upload whose
build number it has seen before, so a rejected build cannot be resubmitted under the same one, and
tying it to the run number means it goes up on its own.

### The listing and TestFlight

The store copy lives in `appstore/metadata` as one text file per field, and
`scripts/appstore-listing.rb` pushes it. Keeping it in files rather than in the script means
changing a description is a diff someone can read, and the listing is reviewable next to the code
it describes. The script checks each field against Apple's length limit before sending, because
Apple rejects an over-length field with a validation error that never mentions the number.

The store version is taken from `tauri.conf.json`, the same place every other version in the repo
comes from. This matters because App Store Connect rejects a build whose `CFBundleShortVersionString`
does not match the version it is uploaded against, and a record created by `produce` starts life at
1.0 regardless of what the app actually is.

Release notes are the one field it will not write on a first version. They describe what changed
since the last release, so Apple refuses them when there is no last release, and there would be
nothing truthful to say.

`scripts/testflight-setup.rb` does the beta side: the blurb testers read, the details Beta App
Review asks for, and the two groups. None of it needs a build to exist, so it can all be in place
before the first upload. Internal testers get builds within minutes of processing. External testers
go through Beta App Review, which needs a contact phone number in
`appstore/metadata/review_phone.txt`; without it the script says so and carries on, because
internal testing does not need it.

`scripts/appstore-review-detail.rb` writes the store submission's App Review Information: the same
contact details and the notes in `appstore/metadata/review_notes.txt`. That is a different record
from the TestFlight one, and the two do not share anything. An explanation that only went to
TestFlight is invisible both to the reviewer looking at the store submission and to the automated
check that runs before a human sees it at all, which is how the first submission was rejected.

`scripts/appstore-compliance.rb` answers the age rating questionnaire and declares App Privacy.
Every content answer is NONE and the privacy answer is that nothing is collected, which is true:
there is no telemetry, no account and no server. The Drive backup sends bytes to the account of the
person who switched it on, which is not the developer collecting anything. If that ever stops being
true, that script is the thing that has to change with it.

`scripts/appstore-screenshots.rb` uploads the frames in `appstore/screenshots`, replacing whatever
is already attached rather than adding to it, so a listing cannot quietly accumulate ten frames
across five runs. How the frames themselves are built is in
[appstore/screenshots/README.md](../appstore/screenshots/README.md); they are a CSS rebuild of the
app rendered at 2560x1600, not a screen capture, because the only Macs to hand have 1x displays and
an upscaled capture looks like one.

Screenshots gate submitting for review, and nothing else: not the app record, not a build upload,
and not either tier of TestFlight.

`scripts/mas-upload-local.sh` builds, signs, packages and uploads from a developer's own machine
rather than CI, using a keychain that exists only for the length of the run. `scripts/testflight-release.rb`
then waits for Apple to finish processing, assigns the build to the external group, submits it for
beta app review, and attaches it to the store version.

That last step is easy to miss and does not announce itself. Assigning a build to TestFlight and
attaching it to the store version are separate operations, and a Mac listing takes its app icon
from the attached build. Until it is attached, App Store Connect shows the listing with no icon and
does not say why.

The Apple ID this is driven with can see more than one App Store Connect team, and the other one
belongs to somebody else, so both scripts pin `FASTLANE_ITC_TEAM_ID` rather than letting spaceship
choose. Do not remove that.

### What the sandbox costs

The App Store build declares four entitlements, in `src-tauri/entitlements.mas.plist`, and each one
is there for a reason worth being able to defend in review. `network.client` is the Google Drive
API. `network.server` is the loopback listener the Drive OAuth flow redirects to, which is the only
installed-app flow Google still supports.

`network.server` is not a theoretical risk. An automated check rejects any submission that declares
it, before review, unless the App Review Information says what listens and why, so
`appstore/metadata/review_notes.txt` explains the loopback bind first and at length: that it is on
127.0.0.1 and never a routable interface, that it lives only for the duration of a sign-in, that it
times out, and how to reach the feature in the app. A rejection on this also has to be answered in
Resolution Center by hand, since that is not in the App Store Connect API.

`files.user-selected.read-write` covers EPUB import and PDF and EPUB export, all of which go
through a panel, so the app only ever reaches the one file that was pointed at. The library needs
nothing: it lives in the container.

Three things are different in that build, and all three are Apple's rules rather than choices:

The updater is gone. `lib.rs` registers the updater plugin only when the config declares it, and
only `tauri.release.conf.json` does, so building against `tauri.appstore.conf.json` leaves it out
by construction. The "Check for Updates" menu item is gated on the same condition, because a menu
item that errors when clicked is worse than an absent one and is its own rejection risk.

The library moves. Sandboxed, `app_data_dir()` resolves inside
`~/Library/Containers/studio.margin.app`, not `~/Library/Application Support`. Someone who switches
from the direct download to the App Store copy sees an empty library. There is no migration and no
plan for one; if that ever matters to a real person it is a first-run import, not a sync.

System spelling additions are gone. `proofing.rs` reads `~/Library/Spelling/LocalDictionary` to
pick up words added elsewhere on the Mac, and the sandbox denies it. The read already fails
quietly, so the app is fine, it just knows fewer of your words.

## Certificates

Three, and they do different jobs. **Developer ID Application** signs the direct download and is
what notarization checks. **Mac App Distribution** signs the App Store bundle. **Mac Installer
Distribution** signs the `.pkg` that wraps it. A Developer ID certificate is not accepted by the
App Store and a Mac App Distribution one will not notarize, so there is no combining them.

`scripts/apple-provision.rb` creates all three, registers the App ID, and makes the provisioning
profile, driving the Developer Portal through fastlane's spaceship rather than the website. It is
find-or-create throughout, so rerunning it is safe. That matters most for the Developer ID
certificate: an account may hold only a handful, they cannot be un-revoked, and every copy of the
app already signed by one stops verifying if it goes away.

It has to be run by a person, not by CI or an agent, because it prompts for the Apple ID password
and a two-factor code:

```
APPLE_EMAIL=you@example.com BUNDLE_ID=studio.margin.app APP_NAME=Margin ruby scripts/apple-provision.rb
```

The private keys live in `~/.margin-signing` and are never sent to Apple, the way the updater key
lives in `~/.tauri`. They were generated with `openssl` up front so that Apple only ever sees a
certificate signing request, and so the `.p12` that CI uses can be assembled locally. Each `.p12`
carries Apple's intermediate certificate alongside the leaf, because a fresh CI keychain holding
only the leaf fails to build a chain to the root and `codesign` stops with an error that does not
say so.

`scripts/apple-secrets.sh` then pipes all of it into the repository's Actions secrets without
printing any of it. The one secret it cannot produce is `HOMEBREW_TAP_DEPLOY_KEY`, which is set up
alongside the tap rather than by anything Apple issued.

One step has no API and has to be done on the website: an App Store Connect API key, under Users
and Access, Integrations. The `.p8` downloads exactly once. Put it at `~/.margin-signing/AuthKey.p8`
with its key ID and issuer ID in `AuthKey.env` beside it, and rerun the secrets script.

Certificates expire after five years and are replaceable; the keys are not backed up anywhere else,
so back them up somewhere that is not this machine.
