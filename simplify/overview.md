# Overview

Four apps, one hand, one design language, four copies of everything underneath. This is what the
audits found, what the duplication has already cost, and the shape of the answer.

## The evidence

Margin, Margin Calendar, Margin Docs and Margin Mail are 101,000 lines of front end and 62,000 lines
of Rust between them. Eleven audits read all of it. The numbers below are measured, not estimated:
where two files were compared, the comparison was a diff or a hash, not an impression.

**1,552 of Margin Calendar's 1,675 non-test OAuth lines exist verbatim in Margin Mail.** Not similar,
verbatim. `auth.rs` 810 of 899, `browser.rs` 416 of 420 where the seven differing lines are all
comment prose, `secrets.rs` 326 of 356, `build.rs` 29 of 29. Six things genuinely differ, and they are
exactly the parameters a shared crate would take.

**73 distinct lines of `lib.rs` are identical in all four apps**, which counting occurrences is 114 to
124 lines per app, between 32% and 48% of each file.

**Margin Mail's `src/ui` is already the shared component library**, seventeen primitives behind one
barrel with a 613-line page rendering every one in every state in both palettes. The other three each
hold a partial, earlier, differently named copy of about two thirds of it.

**Three `Icon.tsx` files are byte identical** (md5 `0ec1a568818f20ed8eed8ad46fbaa2b1`), as is
`escape.ts` in all four, as are two `useToast.ts` files, as are the first 41 lines of two `ipc.ts`
files including the doc comments.

**The command matcher is copy-pasted three times character for character**, down to the local
variable names `needle`, `hay` and `at`.

**Margin Calendar's token file is 49 of 52 values identical to the shared set** it does not depend on.

## What it has already cost

The cost is not the duplicated lines. It is that a fix made once is a fix made in one place.

The same `--ink-faint` contrast problem was found and fixed independently in Margin Calendar and
Margin Mail, both landing on `#6e675b` light and `#8e8677` dark, with Mail's comment citing Calendar's.
Margin and Margin Docs still ship the failing value.

Margin Docs moved a glyph's crossbar from x=5 to x=7 and wrote down why: it "sat left of centre in a
round button". Margin still has the uncentred version.

Margin Docs cut nine static font instances for PDF export and documented the reason at
`pdf.rs:24-31`. Margin still loads variable files into Typst, which lays out at the default instance,
so **every heading and every bold run in a Margin PDF exports at weight 400**. Its Typst escaper is
`JSON.stringify`, so escapes Typst does not recognise reach the page as visible backslashes.

Margin Calendar and Margin Mail both seal their Google refresh tokens with XChaCha20-Poly1305 and both
`Cargo.toml` files carry near-identical comments explaining why `keyring` was rejected. Margin holds
the token for the same OAuth client, on the same grant, **in plaintext**.

Margin Mail wrote the retry and quota layer that Margin Calendar visibly lacks. In Calendar a 429
becomes a generic error, the outbox counts it as a real attempt, and five of them retire the user's
write permanently.

Margin Mail solved the CI checkout problem that Margin Docs still has. **Six of Margin Docs' last
seven CI runs failed**, in `pnpm install`, before running anything.

The `max-parallel: 1` constraint on the release matrix was written down once, with its reason, and
then lost. No workflow in any app sets it today.

That is the pattern, seven times over: one repo learns something, writes it down, and the other three
keep the defect. Consolidation is not tidiness here. It is the mechanism that makes a fix apply once.

## What is broken right now

Three things are not duplication, they are outages, and they are in
[migration.md](migration.md) as preconditions rather than phases.

Margin Docs' CI has been red since the shared dependency landed, because its manifest points at
`file:../../python/margin/shared`, a path that walks out of the repository into a sibling checkout,
and its workflow does a single checkout. That dependency is also unhashed, so a frozen-lockfile
install consumes whatever is on disk, uncommitted edits included.

Margin Mail cannot compile for mobile. Its `mobile_entry_point` attribute sits on the wrong function,
and three helpers its mobile path calls are defined nowhere in the crate. The call sites were copied
from Margin Calendar; the definitions were not. Desktop is unaffected, which is why nobody has hit it.

Margin Mail has no git remote and 123 uncommitted files on one scaffold commit. Margin Docs has 123
uncommitted files.

## The shape of the answer

A fifth repository holding npm packages and Rust crates, MIT licensed so all four apps can consume it
whatever their own licence says, consumed by version rather than by relative path.
[repo-layout.md](repo-layout.md) has the package list and the mechanism.

A monorepo would be simpler, and that is said plainly in `repo-layout.md` rather than implied. The
four apps ship separately, release separately, have separate App Store records and are under two
different licences, so separate repos is the decision. The cost is a two-step for every shared change,
and [risks.md](risks.md) says what that costs and what the escape hatch is.

## What is deliberately not shared

This is the part that stops a consolidation becoming a worse abstraction than the thing it replaced.
Every refusal below is backed by measurement in its own document.

**The sync engines.** Margin Calendar's and Margin Mail's agree on the poll loop, the sink trait and
the event names, and on nothing below that. The cursor models differ in kind, the commit points are
deliberately opposite, recovery is opposite, and conflict resolution is opposite: one uses etags and
`If-Match`, the other declarative replay-safe writes. The trait that looks shared in Mail names no
Google type and has three implementations; Calendar's names Google Calendar types in every signature
and has one. A shared engine would be a larger abstraction than either thing it replaced.

**A settings crate, an error crate, a filesystem crate, an async crate.** All 165 Tauri commands
across all four apps already return `Result<T, String>`, with no `thiserror` or `anyhow` anywhere, so
there is nothing to unify. Three apps keep preferences in `localStorage`. The three atomic-write
implementations are genuinely different and each divergence is justified in a comment.

**The Settings shell.** Generic chrome is 9% of Margin Mail's 2,551-line Settings file. Under 200
lines saved across four apps out of 3,402, and the two consumers disagree about the header, the close
button and the drag region.

**Any shared editor or tiptap extension list.** Three content types, three schema policies, three
lifecycles, and one of the apps has a written argument against the list specifically at the top of its
own extensions file.

**The Typst preambles.** A book and an A4 document, measured at roughly 2% overlap.

**Spinners.** Four different product positions, not four copies of one. Margin Mail bans them on the
record; Margin Calendar has none.

## Where to start

[migration.md](migration.md) has the sequence. The first phase is worth naming here because it is
small and it pays for itself immediately: move the existing shared package into its own repo, change
nothing about its contents, and point all four apps at it including the one that has never depended on
it. That single step makes a fresh clone of every app build, which two of them cannot do today, and it
fixes a CI pipeline that has been red for weeks.

Everything after that is optional in the sense that the apps keep working without it. This first step
is not.
