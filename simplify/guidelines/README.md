# Guidelines

The rules the four Margin apps are built by, written down so they live in the repo instead of in an
assistant's memory files. Everything here was learnt the expensive way on one of the four apps and
then found to apply to all of them.

Read [working-together.md](working-together.md) first if you are an agent picking up a task. Read
[design-language.md](design-language.md) and [errors-and-feedback.md](errors-and-feedback.md) before
touching UI. Read [distribution.md](distribution.md) before touching a release.

- [working-together.md](working-together.md): how work gets done, verified and handed back
- [git.md](git.md): commits, branches, staging, what never goes in a message
- [prose-and-docs.md](prose-and-docs.md): the writing rules, for docs, app copy and chat
- [code-style.md](code-style.md): comments, formatting, tests, the rules that changed
- [design-language.md](design-language.md): warm paper, keyboard first, what the UI never does
- [errors-and-feedback.md](errors-and-feedback.md): quiet failures, loud logs, no silent waits
- [platform.md](platform.md): cross-platform first, and the macOS facts that cost a day each
- [distribution.md](distribution.md): channels, licences, signing, updater, Google Cloud
- [app-facts.md](app-facts.md): the things true of exactly one app

Each rule states what to do and why. The why matters more than the rule: a rule whose reason has
expired should be changed, and several here already have been.

## Where these came from

Assistant memory files under `~/.claude/projects/*/memory/`, the `CLAUDE.md` files in Margin and
Margin Calendar, and the global `~/.claude/CLAUDE.md`. The raw source is preserved verbatim in
`../.research/memories-raw.md` so a claim here can be checked against what was actually said.

Two of the four apps had no memory directory and no `CLAUDE.md` at all, which is the point: Margin
Docs and Margin Mail were operating on rules nobody had written down.
