# Simplify

A plan for turning four separate Margin apps into one family with shared foundations, and the
guidelines that already govern all four written down in the repo instead of in memory files.

Nothing here is built yet. This directory is the design and the sequence.

- [overview.md](overview.md): what is duplicated, what it costs, what the answer is
- [repo-layout.md](repo-layout.md): where shared code lives and how four repos consume it
- [design-system.md](design-system.md): tokens, fonts, icons, the CSS foundation
- [ui-kit.md](ui-kit.md): the shared React primitives
- [hooks.md](hooks.md): the shared hooks, utilities and the IPC wrapper
- [rust-crates.md](rust-crates.md): the shared Rust crates
- [typesetting.md](typesetting.md): the Typst, font and proofing pipeline Margin and Margin Docs both carry
- [accounts.md](accounts.md): Google OAuth, sealed secrets and the sync engines
- [toolchain.md](toolchain.md): tsconfig, Vite, justfile, the developer loop
- [release.md](release.md): CI, signing, notarisation, the updater, distribution
- [testing.md](testing.md): the dev fixture harness and the Playwright suites
- [naming.md](naming.md): the names that disagree with each other, and what to do about it
- [migration.md](migration.md): the order the work happens in
- [risks.md](risks.md): what not to share, and what could go wrong

[guidelines/](guidelines/) holds the rules the apps are built by, harvested from assistant memory
and the scattered `CLAUDE.md` files. Start at [guidelines/README.md](guidelines/README.md).

`.research/` holds the raw audit notes the plan was written from. They are working papers, not
deliverables, and they are where to check a claim.
