# Prose and docs

Applies to everything written: documentation, code comments, app copy, commit messages, PR bodies,
App Store text, website copy and replies in chat.

## Never use an em dash or an en dash

Not `—`, not `–`, anywhere. Use a comma, a colon, a semicolon, brackets, or a full stop and a new
sentence. Pick the one that fits the sentence: swapping the character mechanically produces comma
splices and broken headings.

When editing existing copy, sweep for both characters and replace them.

## Never draw a directory tree

Not in a README, not in a doc, not in a comment, not in a chat reply. A tree is stale the day
somebody adds a file, and anyone who wants the layout can look at it. Name the specific path that
matters, `docs/setup.md`, and move on.

## READMEs

A README is the project description and nothing else. Under 15 lines: what the project is, what it
does, links to the docs.

Setup, usage, internals and design each get their own file in `docs/`. No features list restating
the description, no emoji headings, no badges, no contributing boilerplate.

A long, exhaustive, everything-on-one-page README is the clearest tell of AI-generated code. People
are happy to use AI. They do not want their repo to look like it.

## The docs set

Margin Calendar settled the shape and the other apps followed it. A Margin app's `docs/` holds
`architecture.md`, `conventions.md`, `design.md`, `setup.md` and `release.md`, plus whatever the
product needs (`features.md`, `keyboard.md`, `settings.md`, `mobile.md`, `publishing.md`).

Put detail where someone would go looking for it, not in the first file they open.

Never prefix file names with numbers. `docs/setup.md`, not `docs/01-setup.md`.

## Voice

Prose a colleague would write. Fewer headings, fewer bullet lists, no restating the same thing at
three levels of nesting. No padding and no reassurance.

## App copy

Copy never names a platform. Not "macOS", not "Windows". These ship on Linux and phones too, and a
message that names one OS is wrong on the others. Say "System Settings" or "the system asks once".

Copy never carries a third-party mark or logo. No Google button, no provider logos. The design
language has no room for someone else's brand.

## App Store review notes

The reviewer has the built app and nothing else. Never cite a source file, a line number or a repo
path. Describe what a reviewer can see and do: the UI path to the feature, what it does, the
observable constraints (bound to loopback only, times out, off until an account is connected).

The apps being source-available does not help, because nothing in the notes points at the repo. A
path is noise in a field with a 4000 character limit.

## The checker

`scripts/docs-check.mjs` in Margin Mail enforces the dash rule, the no-trees rule and dead links.
It exists in exactly one repo, and Margin Calendar and Margin Mail are clean while the other two are
not. Margin's remaining offences, checked on 2026-09-06, are seven dashes in `website/README.md` and
three source files that put one in user-visible copy: `src/components/Library.tsx`,
`src/components/ExportPreview.tsx` and `src/export/run.ts`.

Note that the checker only reads `.md`, which is exactly why those three went unnoticed. The plan in
[../naming.md](../naming.md) moves it into the shared toolchain, teaches it to read source files,
fixes its link regex firing inside inline code spans, and gives it a skip list so Margin Docs'
markdown test fixtures do not count against it. Until then, sweep by hand.
