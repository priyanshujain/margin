# App facts

Things true of exactly one app. Everything in the other files applies to all four.

## Margin, the writing studio

`python/margin`, package `margin-app`, bundle id `studio.margin.app`, version 0.1.17, FSL-1.1-MIT.
The oldest and most polished of the four, and the one that ships first on any new channel.

Library data is self-contained `{book-id}.margin` JSON files with images embedded as base64, plus
`custom-dictionary.txt`, in `~/Library/Application Support/studio.margin.app/library/`. That
simplicity is what made the Drive backup feature small.

Google Drive backup is built and lives in `src-tauri/src/gdrive.rs`, with `src/backup.ts`,
`src/store/useBackup.ts`, `BackupButton` and the `BackupSettings` panel on the front. Loopback plus
PKCE through the system browser, scope `drive.file` and `openid email`. `drive.file` is
non-sensitive, so there is no Google verification review and no CASA audit, and the consent screen is
set to Production to avoid the unverified warning and the seven-day refresh token expiry that testing
mode imposes.

The Drive layout is a visible `margin/` folder, one file per book, latest only, because Drive keeps
revisions itself. Backup triggers are manual, on app close, and every 15 minutes, all gated on a
dirty check so nothing uploads unless something changed. The cloud icon opens the panel rather than
backing up in one click, because the panel is the single home for back up, restore, disconnect and
account. Restore is offered as an ignorable link on the empty-library state, which doubles as
connect-on-a-new-machine.

The refresh token and sync state sit in plaintext in `backup.json` in the app data directory. That
was a deliberate step back from the keychain, for the reasons in [code-style.md](code-style.md), and
it matches the app's existing local-data model given the limited scope. The newer apps seal theirs
with XChaCha20-Poly1305 instead, and Margin should be brought up to that.

Margin has no test script at all. It is the only one of the four without one.

`appstore/` and `target-mas/` are the Mac App Store build track.

## Margin Calendar

`python/margin-caledar`, package `margin-calendar`, bundle id `studio.margin.calendar`, MIT. The
directory name really is misspelt.

Its `docs/design.md`, `docs/conventions.md` and `docs/architecture.md` are the model the rest of the
suite documents by. When writing docs for another app, follow those.

Prev and next move one day, never a week. Week view is a rolling seven days from the anchor by
default, with `weekMode()` in `src/time.ts` stored under `margincal-week-mode` and `weekAnchor()` as
the single source both `spanFor` and `GridView` use. Never reintroduce a bare `startOfWeek` call in
either. The snapping "calendar" mode exists in Settings, and only there do the arrows step by a week,
because a day step would be invisible.

The vertical axis has contraction hysteresis for the same reason the horizontal one slides by a day:
positional memory is most of the speed of a keyboard-driven calendar.

Persisted localStorage keys are `margincal-folds`, `margincal-bounds`, `margincal-view` and
`margincal-theme`. See [platform.md](platform.md) for how to read them out of the installed app.

This is the app with the Nix flake, and the one whose Linux story the others should copy.

## Margin Docs

`rust/margin-editor`, package `margin-docs`, remote `margin-docs`, MIT. Three names for one thing,
and the directory is the odd one out.

No memory files and no `CLAUDE.md` exist for this app, so nothing was ever written down about it. It
is also the largest front end in the suite at 43,000 lines, and it is a near twin of Margin on the
Rust side: `pdf.rs`, `fonts.rs`, `macspell.rs` and `writingtools.rs` exist in both. That duplication
is the subject of `../typesetting.md`.

## Margin Mail

`rust/margin-mail`, bundle id in the same `studio.margin.*` family, FSL-1.1-MIT. Third in the suite
and the largest Rust codebase at 46,000 lines. No git remote yet.

The premise is a keyboard-first email client over Gmail where the user never feels Gmail. HEY and
Superhuman are the feature vocabulary. Mailspring is the reliability bar.

Product decisions locked on 2026-09-03, not to be reopened unless the user does:

macOS first, then iOS, then minor work for Linux. List plus reading pane in the Superhuman shape,
with HEY's Reply Later and Set Aside piles, and Feed and Paper Trail as views. The Inbox is one list
in time order under Back, with unseen shown by weight alone: no dot, no band, no groups. Seen on open
at once, a new reply makes a thread unseen again, the dock badge stays, and there are no counts in
the list. The Screener is on by default with a first-run pass that screens in anyone who has emailed
before. No AI in v1, though the design leaves room. Trackers are blocked and never sent, because
privacy and ownership are core tenets. Scheduled sending is skipped; snooze and Bubble Up stay,
evaluated lazily when a device opens or wakes. Multiple accounts with per-account views and an
optional unified view. Backends after Gmail are generic IMAP and SMTP, then JMAP for Fastmail.
Calendar invites RSVP inline and hand off to Margin Calendar, with no calendar sidebar. Compose is
inline at the end of the thread, or a floating card for new mail. Full local mirror of mail, with
attachments on demand.

App state must not depend on Gmail or any provider. In the user's words: "if I switch to protonmail
tomorrow I don't want to lose data." Local data is the truth and cloud stores are backup only, behind
one interface, with Google Drive for non-technical friends and Cloudflare R2 for the user. Key
portable state on the RFC Message-ID and the sender address, never on provider ids.

Four Playwright specs fail on a clean tree and are not flakes. `tests/shell.spec.ts` ("New for you
above Previously seen"), `tests/snooze.spec.ts` ("Back above New for you") and `tests/triage.spec.ts`
("Mark all as seen is a link on the heading") all assert Inbox group heads that the app deliberately
no longer draws: `GROUPS` in `src/ipc.ts` has no `new` or `seen` label and `ListColumn` renders
`GroupHead` with no action. The fourth, `tests/kit.spec.ts` ("every text field tells the webview not
to fill it in"), is a static scan that flags an input in `Kit.tsx` and one in `Settings.tsx`. Match on
the test name, not the line number. If exactly these fail, say they are pre-existing and move on.
Fixing them means rewriting the assertions to the current design, which is its own piece of work.
