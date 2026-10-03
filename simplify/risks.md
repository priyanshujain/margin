# Risks

What could go wrong, what should not be shared at all, and the cost of the shape chosen in
[repo-layout.md](repo-layout.md).

## The two-step tax

Four separate repos consuming a fifth means every shared change is two commits and a version bump.
Today a token can be changed and seen in an app on the next dev server restart, because the
dependency is a symlink into a sibling checkout. After consolidation that becomes: change the shared
repo, tag it, bump four manifests, install four times.

That is a real regression in the inner loop and it is the price of the shared code being correct
everywhere else. The mitigation is a documented local override, `pnpm.overrides` for npm and a
`[patch]` or `.cargo/config.toml` `paths` entry for Rust, so day to day work on shared code is still
a live edit. The rule is that the override is a switch a developer turns on, never a value the repo
ships, because an override committed by accident reintroduces exactly the problem being fixed.

If in six months the two-step is the main friction, the honest answer is a monorepo, and that
conclusion should be allowed rather than argued around.

## Version skew across four consumers

Independent apps on independent release cadences will sit on different shared versions. That is
normal and fine until a shared change is not backward compatible, at which point one app is broken
and nobody notices because it was not the app being worked on.

The countermeasure is the smallest one that works: the shared repo's own CI builds all four apps
against the candidate before a tag is cut. That needs the four repos to be checkoutable from CI,
which needs Margin Mail to have a remote, which is the first item in [migration.md](migration.md).

## Sharing things that only look alike

The audits found several places where two implementations resemble each other at the top and diverge
completely underneath. Extracting those produces an abstraction larger than either thing it replaced,
with a configuration surface nobody can reason about.

The clearest case is the sync engines. Margin Calendar and Margin Mail agree on the poll loop, the
sink trait and the event names, and agree on nothing below that: the cursor models differ in kind,
the commit points are deliberately opposite, recovery is opposite, and conflict resolution is
opposite. One uses etags and `If-Match`, the other uses declarative replay-safe writes. The trait
that looks shared in Margin Mail names no Google type and has three implementations; the calendar's
names Google Calendar types in every signature and has one. A shared engine here would be a mistake
and the plan says so.

The same judgement applies to spinners and loading states, which are four different product
positions and not four copies of one, to badges, to the floating editor toolbar, and to each app's
Typst preamble, which the measurement put at roughly 2% overlap between a book and an A4 document.

Sharing is justified by measured duplication, not by two files having the same name. Every
extraction in this plan carries a number.

## Sharing too early

`@margin/ui` is the package with the most value and the most risk, because a component API extracted
from two call sites is usually wrong. The rule for it: a primitive moves into the kit when three of
the four apps want it, or when two want it and the third's absence is a bug. Anything with two
honest consumers stays where it is until a third appears.

## Freezing a bug into the shared copy

Three of the audits found that when two files drifted, one repo moved ahead and the other kept a
defect the first had already found and written a paragraph about. Margin's PDF export ships every
heading at weight 400 because it loads variable fonts where Margin Docs cut static instances; its
Typst string escaper uses `JSON.stringify`, so escapes Typst does not recognise reach the page as
visible backslashes; its Google refresh token sits in plaintext where two siblings seal theirs.

Extraction must take the better version, not the older one or the one in the repo being worked in.
Where the two differ, the plan names the winner explicitly, and the losing behaviour has to be
verified as gone rather than assumed. The reverse risk is worse: standardising four apps onto the
oldest implementation would make three of them worse at once.

## The licence conflict is load-bearing

Margin is FSL-1.1-MIT and Margin Mail will be. Margin Calendar and Margin Docs are MIT. Shared code
cannot be under two licences, and the shared package currently lives inside the FSL repo. The plan
makes the shared repo MIT, which every app can consume, but that is a decision with consequences and
it should be made deliberately rather than discovered during extraction.

## Uncommitted work blocks repo surgery

Margin Docs has 123 uncommitted files. Margin Mail has 123 uncommitted files on top of a single
scaffold commit and has no remote at all. Moving files between repos, rewriting manifests and cutting
tags across that is how work gets lost.

Nothing in this plan starts until those two trees are committed and pushed. That is not a
precaution, it is a precondition.

## Design drift is the thing being fixed, so measure it

The reason for all of this is that the same fix keeps being made four times, and the evidence is
specific: eleven independent implementations of a square icon button, five different fixes for the
same icon baseline problem, the same `--ink-faint` contrast correction discovered independently in
two apps while the other two kept the failing value, and a centring fix for one glyph that exists,
with a written explanation, in exactly one repo.

After consolidation, that class of drift should be impossible rather than merely discouraged. The
check that makes it so is a lint over each app's CSS for colour literals outside the token layer and
for glyph paths defined outside `@margin/icons`. Without it, the tokens get shared and the drift
resumes in the app stylesheets within a month.

## What this plan does not fix

It does not make the four apps one product, and it should not try. They have different data models,
different backends, different keyboard vocabularies and different reasons to exist. The goal is that
they share a foundation and a look, not that they share a shape.
