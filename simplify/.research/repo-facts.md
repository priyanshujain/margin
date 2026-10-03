# Repo facts (gathered directly, 2026-09-06)

Ground truth for the other research notes. Everything here was read off disk, not inferred.

## The four repos

| App | Directory | Git remote | Commits | Uncommitted files |
|---|---|---|---|---|
| Margin (writing studio) | `python/margin` | `git@github.com:priyanshujain/margin.git` | 149 | 1 |
| Margin Calendar | `python/margin-caledar` | `git@github.com:priyanshujain/margin-calendar.git` | 27 | 0 |
| Margin Docs | `rust/margin-editor` | `git@github.com:priyanshujain/margin-docs.git` | 8 | 123 |
| Margin Mail | `rust/margin-mail` | none configured | 1 | 123 |

All paths are relative to `/Users/pj/Workspace/projects`.

Three names disagree with themselves. The calendar's directory is misspelt (`margin-caledar`), the
docs app lives in a directory called `margin-editor` while its package is `margin-docs` and its
remote is `margin-docs`, and the two Rust-family apps sit under `rust/` while the two older ones sit
under `python/` for no reason that survives inspection. None of the four is a Python project.

Margin Mail has no remote and a single scaffold commit with 123 files of uncommitted work on top of
it. Margin Docs has 123 uncommitted files. Any plan that starts by moving files between repos has to
deal with that first: see the sequencing note in `../migration.md`.

There is no separate website repo on disk. The Astro site lives at `python/margin/website`.

## Size

| App | TS/TSX/CSS files | TS lines | Rust files | Rust lines |
|---|---|---|---|---|
| Margin | 72 | 10,871 | 12 | 2,190 |
| Margin Calendar | 100 | 15,517 | 20 | 8,490 |
| Margin Docs | 151 | 43,311 | 14 | 5,365 |
| Margin Mail | 172 | 31,409 | 98 | 45,884 |

About 101,000 lines of front end and 62,000 lines of Rust across the four.

## The shared package as it stands

`python/margin/shared` is a real npm package named `margin-shared`, tracked in Margin's git repo
(26 files). It has no build step and no dependencies: consumers resolve its TypeScript source
directly through Vite. It contains `css/tokens.css`, `css/fonts.css`, `src/fonts.ts` (233 lines),
`src/icons.ts` (61 lines), `src/index.ts`, twelve variable font binaries with their licences, and
`bin/sync-fonts.mjs`, which copies those binaries into a consuming app's `public/fonts`.

Margin depends on it as `"margin-shared": "file:./shared"`. Margin Docs and Margin Mail both depend
on it as `"margin-shared": "file:../../python/margin/shared"`, a path that walks out of the
repository and into a sibling checkout. On this machine pnpm has resolved that to a symlink and it
works. On a fresh clone it does not: `pnpm install` in Margin Docs fails unless Margin happens to be
checked out at exactly that relative location, which no CI runner and no other person will reproduce.
Margin Calendar does not depend on it at all and carries its own copies of the same tokens.

This is the single fact that motivates the whole exercise. The shared package is the right idea
executed in a way that only works on one laptop.

## Toolchain in use

pnpm 10.12.4, Node 25.5.0, cargo and rustc 1.96.1. React 19.1, Vite 7, TypeScript 5.8, Tauri 2,
zustand 5 in all four apps. No prettier anywhere. No eslint anywhere.

## CLAUDE.md files

Only Margin and Margin Calendar have one, 16 lines each. Margin Docs and Margin Mail have none, so
everything those two projects have learnt lives in assistant memory files outside the repos, which is
exactly what `guidelines/` is for.
