# Names and documentation conventions

Three of the four apps disagree with themselves about what they are called, and the rules they all
follow are written down three times with three sets of edits. This is the plan for settling both.
The evidence is in [.research/docs-conventions.md](.research/docs-conventions.md) and
[.research/repo-facts.md](.research/repo-facts.md); the rules themselves are in
[guidelines/prose-and-docs.md](guidelines/prose-and-docs.md) and
[guidelines/code-style.md](guidelines/code-style.md) and are not restated here.

## The names as they stand

Bold marks a cell that disagrees with the others or with itself, and paths are relative to
`/Users/pj/Workspace/projects`.

| | Margin | Margin Calendar | Margin Docs | Margin Mail |
| --- | --- | --- | --- | --- |
| directory | `python/margin` | **`python/margin-caledar`** | **`rust/margin-editor`** | `rust/margin-mail` |
| `package.json` name | **`margin-app`** | `margin-calendar` | `margin-docs` | `margin-mail` |
| Cargo package | **`margin-app`** | `margin-calendar` | `margin-docs` | `margin-mail` |
| Cargo lib | `margin_app_lib` | `margin_calendar_lib` | `margin_docs_lib` | `margin_mail_lib` |
| bundle identifier | `studio.margin.app` | `studio.margin.calendar` | `studio.margin.docs` | `studio.margin.mail` |
| git remote | `priyanshujain/margin` | `priyanshujain/margin-calendar` | `priyanshujain/margin-docs` | **none** |
| `productName` | `Margin` | `Margin Calendar` | `Margin Docs` | `Margin Mail` |
| window title | **`margin`** (`index.html:27`) | `Margin Calendar` (`:41`) | `Margin Docs` (`:55`) | `Margin Mail` (`:38`) |
| README h1 | **`# margin`** | `# Margin Calendar` | `# Margin Docs` | `# Margin Mail` |
| name in docs prose | **`margin`, lowercase** | `Margin Calendar` | `Margin Docs` | `Margin Mail` |
| storage prefix | `margin-` | `margincal-` | `margindocs-` | `marginmail-` |
| licence | FSL-1.1-MIT | MIT | MIT | FSL-1.1-MIT |

The disagreements, worst first:

1. `python/margin-caledar` is a typo. Everything inside says `margin-calendar`: the package, the
   crate, the remote, and the Nix flake output that CI builds (`flake.nix:14,18,19`,
   `.github/workflows/ci.yml:81` runs `nix build .#margin-calendar`).
2. `rust/margin-editor` is the only place the word "editor" survives as a name. Package, crate,
   bundle id, `productName` and remote all say docs.
3. `python/` and `rust/` are wrong for all four. Every one is a Tauri 2 app with a React front end
   and a Rust backend; none is a Python project. This is not cosmetic: `margin-docs/package.json:33`
   and `margin-mail/package.json:27` both carry
   `"margin-shared": "file:../../python/margin/shared"`, and `margin-mail`'s CI checks two repos out
   into `rust/margin-mail` and `python/margin` (`ci.yml:25,30`) purely to reproduce that path. The
   word "python" is baked into a GitHub runner's filesystem layout.
4. Margin Mail has no remote. One commit, `088ec9c`, with 123 uncommitted files on top. The name is
   still an open choice, which makes this the cheapest moment to fix the pattern.
5. `margin-app` is the only package and crate name that is not the product name lowercased and
   hyphenated, and it is why the bundle id reads `studio.margin.app`, a namespace with a placeholder
   in it.
6. Margin calls itself `margin` in lowercase in its README h1, its window title (`index.html:27`)
   and in every sibling's prose (`margin-calendar/docs/conventions.md:3`,
   `margin-docs/docs/conventions.md:3`, `margin-mail/README.md:8`), while `productName` is `Margin`.
7. Licences split two and two and neither MIT README says so. [repo-layout.md](repo-layout.md)
   depends on this: the shared repo has to be MIT so all four can consume it.
8. Margin Docs' `docs/conventions.md:3` points at `../margin` and `../margin-calendar`. Neither
   exists relative to `rust/margin-editor`. The paths only make sense once the collapse has run.

Target: one parent, `~/Workspace/projects/margin/`, holding `margin`, `margin-calendar`,
`margin-docs`, `margin-mail` as siblings, with `margin-shared` joining them later.

## The rename plan

### Before anything moves

Margin Docs has 123 uncommitted files and Margin Mail has 123 on top of a single scaffold commit.
Moving a directory does not disturb git, which stores paths relative to the repository root, so that
work survives a `mv` intact; commit or stash first anyway, so a mistake is one `git checkout` away.
What does not survive is everything keyed on the absolute path:

- `node_modules`. pnpm's store directory name encodes the specifier
  (`node_modules/.pnpm/margin-shared@file+..+..+python+margin+shared/`), so both Docs and Mail need
  `node_modules` removed and `pnpm install` rerun after the move, not just a lockfile edit.
- The assistant's per-project memory and session history, under
  `~/.claude/projects/-Users-pj-Workspace-projects-<slug>/`. Five such directories exist, one per app
  plus `margin-website`. Rename each to the new slug or the app's learnt facts are orphaned.
- Shell history, editor workspaces, and any absolute path written into a doc.

### Steps 1 and 2: rename the two misnamed directories

`rust/margin-editor` to `margin-docs`, then `python/margin-caledar` to `margin-calendar`. Both are
one `mv` and free. Nothing references either by name: `file:../../python/margin/shared` is unaffected
because the depth does not change, both remotes are already correct, and the flake output never
mentioned the misspelling. Fix Docs' `docs/conventions.md:3` in the same sitting, which is
disagreement 8.

### Step 3: give Margin Mail a remote

`priyanshujain/margin-mail`, matching the other three, created before the collapse so the CI paths
below are written once. Free now, expensive to change after the first release, because the repo name
is in the release URL the Homebrew cask and the updater fetch from.

### Step 4: collapse `python/` and `rust/` into one `margin/` parent

The expensive step and the one that pays, and best done immediately after step 3, while Mail has no
CI history to invalidate. Exhaustively, what changes:

- `margin-docs/package.json:33` and `margin-mail/package.json:27`:
  `file:../../python/margin/shared` becomes `file:../margin/shared`, an interim value.
  [repo-layout.md](repo-layout.md) replaces it with a published `@margin/*` dependency, so if the
  shared repo lands first, skip this edit entirely.
- Both lockfiles record the specifier in three places each: `margin-docs/pnpm-lock.yaml:54,55,1449,
  1450,3207` and `margin-mail/pnpm-lock.yaml:36,37,918,919,1957`. Regenerate with `pnpm install`,
  never hand-edit.
- `margin-mail/.github/workflows/ci.yml`: the checkout paths at lines 25 and 30 and the
  `working-directory` lines at 21, 57 and 91 lose their `rust/` and `python/` prefixes.
- `margin-docs/.github/workflows/ci.yml`: gains the second checkout it never had, which is the fix
  rather than the cost. See below.
- The local paths listed under "before anything moves".

What must not change: any bundle identifier, any git remote, any release tag, the Homebrew tap or
its cask. Nothing on GitHub is affected by a local move.

The failing build. Margin Docs' `ci.yml:19` does one checkout and `ci.yml:29` runs
`pnpm install --frozen-lockfile`, so `file:../../python/margin/shared` has nothing to resolve to. Run
33308997470 (2026-08-30) failed with
`ENOENT: no such file or directory, scandir '/Users/runner/work/python/margin/shared'`, and five of
the last six runs failed. Margin Mail solved this by checking `priyanshujain/margin` out a second
time; Margin Docs never did. The fix is Mail's two-checkout block with the shorter paths, and since
`priyanshujain/margin` is public the second checkout needs no token.

### Step 5: `margin-app` to `margin`, package and crate only

Do the package and crate rename. Do not touch the bundle identifier. The cost: `package.json:2`,
`src-tauri/Cargo.toml:2` and the lib name at `:15`, `Cargo.lock`, `src-tauri/src/main.rs:5`
(`margin_app_lib::run()`), `.github/workflows/release.yml:47` and `:50` (an awk that bumps the
version by matching `/^name = "margin-app"$/`, which silently stops matching rather than failing),
the artefact name at `appstore.yml:128`, and the Xcode project under `src-tauri/gen/apple/`, which
holds `margin-app.xcodeproj` and a `margin-app_iOS` directory named at `project.yml:1,27,34,40,57`
and is best regenerated with `tauri ios init` rather than renamed by hand.

### The bundle identifier is frozen

`studio.margin.app` stays, along with the other three, and one line in Margin's `docs/conventions.md`
should say so and why: it is the one inconsistency in the table deliberately left standing.

Changing an identifier on a shipped app orphans the application support directory Tauri derives from
it. Every app reads its library through `app_data_dir` (Margin Mail's `src-tauri/src/library.rs:8-9`
is the shared shape), so a new identifier makes an existing install look like a fresh one: accounts,
sealed refresh tokens, the log and the database all move out from under the app. Five other things
are keyed on the same string:

- The sealed secret service name. `margin-calendar/src-tauri/src/google/secrets.rs:41` and
  `margin-mail/src-tauri/src/google/secrets.rs:42` use the identifier as `SERVICE`, and the
  reference is composed from it (`secrets.rs:307` asserts `studio.margin.calendar/1234`).
- The OAuth redirect registered with Google. `google/auth.rs:112` in Calendar and `:136` in Mail
  declare `studio.margin.<app>:/oauth2redirect`, matched by the deep link schemes in
  `tauri.conf.json` (Calendar `:35,41`, Mail `:34,38`). Changing it means re-registering the mobile
  clients in the Google console.
- The macOS notification settings deep link, `margin-mail/src-tauri/src/notify/macos.rs:259`.
- The App Store record. Ten scripts under `margin/scripts/` default `BUNDLE_ID` to
  `studio.margin.app` (`appstore-listing.rb:24`, `apple-provision.rb:40`, `apple-secrets.sh:9`,
  `mas-upload-local.sh:16`, the three `testflight-*.rb` at `:19` and `:21`, and three more), and
  `src-tauri/gen/apple/project.yml:3,14` carries it into the Xcode build. A bundle id is the App
  Store's primary key: a new one is a new app, with no reviews, testers or purchase history.
- The Homebrew cask. `release.yml:231-258` pushes a version and sha into
  `priyanshujain/homebrew-margin`, `Casks/margin.rb`, whose uninstall and zap stanzas name the
  installed bundle.

A note on the updater, which is easy to get wrong by reading the committed config alone. The
`plugins` block in Margin's and Margin Docs' `tauri.conf.json` is empty and Calendar's and Mail's
hold only `deep-link`, but that is the whole point of the overlay described in
[guidelines/distribution.md](guidelines/distribution.md): all four apps carry a
`src-tauri/tauri.release.conf.json` and all four of those configure the updater, because the key's
mere presence in the committed file would make a local `tauri build` demand a signing key. So the
updater is real, and a bundle id change does break the update path for every existing install, which
would no longer recognise the new bundle as itself. It is not a signature problem, it is an identity
one. The five reasons above stand regardless.

## The conventions files

Only three exist: `margin-calendar/docs/conventions.md` (89 lines),
`margin-docs/docs/conventions.md` (103) and `margin-mail/docs/conventions.md` (139). Margin, the app
all three defer to, has none: the house style is written down only in the repos that copied it. Five
rules are identical in all three, word for word:

| rule | calendar | docs | mail |
| --- | --- | --- | --- |
| `Result<T, String>` everywhere, no `anyhow` | 9 | 11 | 9 |
| `dto.rs` is the frozen IPC contract, mirrored by `src/ipc.ts` | 12 | 13 | 14 |
| one zustand store per domain, no middleware, one selector per field | 26 | 26 | 35 |
| flat kebab-case class names, state as `data-*`, never `is-` | 44 | 79 | 84 |
| transitions name explicit properties and use `var(--ease)` | 52 | 88 | 92 |

Each file also opens by disclaiming originality in near-identical words, and each closes with a
`## Never` section of the same shape (calendar:86, docs:100, mail:136).

Where they genuinely contradict each other:

- **Dashes.** Docs:103 bans em dashes and en dashes. Calendar:89 and mail:139 ban only em dashes.
  The house rule bans both, so Docs is right and the other two are stale.
- **Where a token lives.** Calendar:46 and docs:81 say every colour, radius and size goes through a
  token in `src/styles/tokens.css`. Mail:55 redefines that file as a seam rather than a list, and
  mail:86 says add to `src/styles/mail.css` instead. Mail is also the only one with the three-layer
  tokens, primitives, screens rule (mail:51-66).
- **Container queries.** Calendar:67-70 permits exactly one, on the event block, with a reason.
  Mail:105 hardens that to "there is no container query in this repository" while crediting the
  calendar's exception. Docs is silent. Three postures, one subject.
- **Icon buttons.** Calendar:79 and mail:120 both require an icon-only button to carry a `title` with
  its shortcut in real glyphs; docs drops the line. Mail:116 is the only file that names the icon
  module (`src/ui/icons.ts`) and the only one that acknowledges `margin-shared/icons`, a real
  dependency of Margin Docs.
- **Comment density.** Calendar:22 and mail:31 both say "Comments are rare and explain why, never
  what. Match the density in `lib.rs`." Docs:22 keeps the sentence and drops the pointer. The claim
  is false in all three by a factor of ten to twenty.

The error carve-outs are not a contradiction and are the pattern to keep: calendar:9 allows
`google::api::ApiError` because the sync engine has to tell a 410 from a 412, mail:9 allows
`provider::ProviderError`, docs:11 allows none. Each names its own reason in the same sentence.

The one dead cross-reference is `margin-docs/docs/conventions.md:3`, which says the project is a
sibling to `../margin` and `../margin-calendar`. From `rust/margin-editor` neither path exists. The
collapse makes both resolve, and the rename should correct the sentence anyway. The docs checker does
not catch it, because it is inline code rather than a markdown link.

The plan is one shared document, in the shared repository. `guidelines/` moves out of
`margin/simplify/` into `margin-shared/guidelines/` when that repo exists
([repo-layout.md](repo-layout.md)), MIT licensed so all four apps can copy from it whatever their own
licence says, and it gains `guidelines/conventions.md` holding the five identical rules verbatim plus
the shared CSS and store rules, so there is exactly one copy of each.

Four separate repositories mean a relative link between them cannot resolve and a URL is not read by
anyone working offline. Use the mechanism that already exists for fonts: `margin-shared/bin/` ships
`sync-fonts.mjs` with a `--check` mode, wired as `fonts:sync` and `fonts:check` in
`margin/package.json:12-13` and `margin-docs/package.json:15-16`. Add `sync-guidelines.mjs` on the
same shape, copying `guidelines/*.md` into each app's `docs/guidelines/` with `--check` failing CI on
drift. The cost is four copies of nine files, acceptable only because the check makes drift loud.

Each app then keeps a `docs/conventions.md` holding only its own rules: Docs' `## Markdown` (38-59)
and `## Tests` (61-75), Mail's `## Places and stages` (68-80) and `## Work packages` (129-134), the
three-layer token rule, the storage prefix, and each error carve-out with its reason. Calendar's
`data-phone` versus `data-touch` argument (57-70) is copied verbatim into mail:94-103, so it belongs
in the shared file, once.

Until the shared repo exists, fix the three files in place: take Docs' dash wording into the other
two, cut "Comments are rare" from all three, and correct Docs' line 3.

One thing not to fix. `margin-docs/src/markdown/corpus/real/` holds 16 markdown fixtures, among them
`calendar-conventions.md`, `editor-conventions.md`, `margin-readme.md`, `margin-claude.md` and
`margin-website-readme.md`. They are snapshots for the serializer round-trip tests and they will
drift from the originals, which is correct: a fixture that tracks a moving file is not a fixture. Say
so in Docs' `## Tests` section and exclude the directory from the checker.

## The canonical docs set

Five files, same names in every app: `architecture.md`, `conventions.md`, `design.md`, `setup.md`,
`release.md`. Then only what the product genuinely has. Never a numeric prefix.

| file | Margin | Calendar | Docs | Mail |
| --- | --- | --- | --- | --- |
| `architecture.md` | **missing** | 145 | 545 | 352 |
| `conventions.md` | **missing** | 89 | 103 | 139 |
| `design.md` | **missing** | 152 | 131 | 153 |
| `setup.md` | **missing** | 55 | 29 | **missing** |
| `release.md` | **missing** | 105 | 117 | 119 |
| product files | `publishing.md` (229) | `mobile.md` (304) | none | `features.md` (475), `ui.md` (339), `settings.md` (201), `plan.md` (193), `keyboard.md` (129), `help.md` (74), `mockups/`, `research/` |

Margin, the originating app, has one document. Margin Mail has ten and is missing the one a new
machine needs, and it is the app that requires a Google OAuth client to run at all. What each of the
five holds, read off the three sets that exist: `architecture.md` opens with the same stack
sentence in all three ("Tauri 2, React 19, Vite, TypeScript and zustand on the front, Rust behind",
calendar:3, docs:3, mail:3), then "The split is strict" and what each side owns, then one section per
hard part, then `## Order of work`. `design.md` argues product decisions as prose under "why"
headings and ends with `## Visual language`. `release.md` runs `## Installing locally`,
`## Cutting a release`, `## What the build needs`, `## Updates`. `setup.md` is what a fresh machine
does: toolchain versions, credentials, the first run.

Margin needs all five written and keeps `publishing.md`. Mail needs `setup.md`, covering the Google
OAuth client, `google-credentials.json` and the fixture harness. Mail's `plan.md` is a milestone
tracker that will go stale, so it moves under `docs/research/` or into the issue tracker. Mail's
README is 20 lines whose lines 3 to 10 are a pitch already written at `docs/design.md:31-73`; cutting
it to two sentences puts it at 13. Margin's README h1 and `index.html:27` become `Margin`.

## The comment style

The rule the code actually follows, as against the rule two repos have written down: **a comment
never says what, and always says why.** Measured over `src/`, `src-tauri/src/` and `shared/src/`:

| repo | source lines | comment lines | share |
| --- | --- | --- | --- |
| Margin | 10,377 | 125 | 1.2% |
| Margin Calendar | 20,211 | 2,208 | 10.9% |
| Margin Docs | 43,985 | 10,634 | 24.2% |
| Margin Mail | 70,737 | 9,921 | 14.0% |

The three densest repos are the three best ones. Margin is not compliant by being sparse, it is
under-commented, and the proof is that the two files sibling repos cite by name as the source of a
decision carry no comment at all:

- `margin/src-tauri/src/gdrive.rs`, 953 lines, zero comment lines.
  `margin-calendar/docs/conventions.md:17` points at `gdrive.rs:286` as the origin of `read_json` and
  explains why the body goes to a `String` first, so the error payload survives into the message.
  That reason is written in two other repositories' docs and nowhere in the file.
- `margin/src-tauri/src/pdf.rs`, 129 lines, zero comment lines. Calendar:20 and mail:23 both cite
  `#[tauri::command(async)]` on a synchronous fn as margin's trick for getting off the main thread
  without hand-writing `spawn_blocking`. `compile_pdf` is at `pdf.rs:90` and says nothing.

Both get a comment naming the decision and the failure the other choice produces. The narration to
delete, specifically:

- `margin/src/components/ExportPreview.tsx:320`: `// render cancelled or page failed; keep the
  previous canvas`. The second clause narrates the line below.
- `margin/src-tauri/Cargo.toml:9`, the `See more keys and their definitions at ...` line, and lines
  12 to 14, the paragraph explaining the `_lib` suffix. Both are `cargo new` boilerplate that the
  three sibling manifests deleted.
- `// Prevents additional console window on Windows in release, DO NOT REMOVE!!` at
  `src-tauri/src/main.rs:1` in all four repos. Tauri scaffold text, shouting, and none of these apps
  ships on Windows.
- Four `// eslint-disable-next-line react-hooks/exhaustive-deps` in Margin
  (`src/components/FindBar.tsx:44`, `src/editor/FloatingToolbar.tsx:69`, `src/editor/Editor.tsx:113`
  and `:122`). No repo has eslint, in `package.json` or on disk. Dead directives.

A sweep for narration patterns across all four repos found almost nothing else: the voice is being
held, and it is the written rule that is wrong.

## The CLAUDE.md problem

`margin/CLAUDE.md:9` and `margin-caledar/CLAUDE.md:9` are byte-identical:

    - Avoid excessive comments. Only comment when absolutely necessary. Code should be readable and
      not require comments to understand it.

Read literally, that instructs anyone picking up the work to strip the best thing in the codebase:
the dependency blocks in Mail's and Docs' `Cargo.toml` that say why a version is pinned exactly and
why `keyring` is not used, and the file heads in `shared/src/fonts.ts` and `shared/src/icons.ts` that
say why one list exists instead of four. It was true of a repo with no siblings and stopped being
true the moment a decision had to survive being copied into another repo. Margin Docs and Margin Mail
have no `CLAUDE.md` at all, so the two most disciplined repos run on rules nobody wrote down.

The replacement, given that `guidelines/` now exists and is the real answer: delete the coding and
prose bullets from both files and give all four repos the same short `CLAUDE.md`, about ten lines,
naming the app and pointing at `docs/guidelines/` (synced by `sync-guidelines.mjs --check`, above)
and at the app's own `docs/conventions.md`, with nothing else in it. The git rules in Margin's and
Calendar's files are correct and already restated in [guidelines/git.md](guidelines/git.md), so they
move rather than being lost. `margin-docs/src/markdown/corpus/real/margin-claude.md` is a snapshot of
Margin's current file used as a test fixture, and it stays as it is.

## The docs checker

`margin-mail/scripts/docs-check.mjs`, 72 lines, wired to `just docs` (`margin-mail/justfile:33-34`)
and to CI (`margin-mail/.github/workflows/ci.yml:51`). It walks every markdown file in the repo,
skipping `node_modules`, `dist`, `target`, `.git`, `gen` and `.playwright-mcp`, and reports three
things: em and en dashes anywhere including inside code fences, directory trees (a run of box-drawing
glyphs, or three or more consecutive lines of the ASCII form), and relative links whose target does
not exist. It exists in one repository, and Margin, which has the most offences, is the one that most
needs it. Run unchanged against each repo today (2026-09-06):

| repo | em | en | trees | dead links | total | outside verbatim material |
| --- | --- | --- | --- | --- | --- | --- |
| Margin | 27 | 0 | 0 | 42 | 69 | 7 |
| Margin Calendar | 0 | 0 | 0 | 0 | 0 | 0 |
| Margin Docs | 7 | 0 | 0 | 46 | 53 | 1 |
| Margin Mail | 0 | 0 | 0 | 0 | 0 | 0 |

The last column is the number that matters. Margin's 69 are almost all inside this plan directory: 46
are in `simplify/.research/memories-raw.md`, a verbatim dump of old memory files that exists so a
claim can be checked and must not be edited, and most of the remaining dead links point at plan
documents not written yet. Margin's only real offences are the seven em dashes in
`website/README.md` (lines 3, 21, 22, 23, 25, 37, 41). Margin Docs' 53 are all in
`src/markdown/corpus/` except one false positive at `docs/architecture.md:151`, where the prose
discusses markdown link syntax and the checker's link regex fires inside a code span, reading a
bracketed target as a path.

Three changes before it can be shared:

1. The link check must ignore fenced blocks and inline code spans. The dash check must not: catching
   a dash inside a fence is deliberate.
2. A skip list for verbatim material, as a `.docsignore` or an exported constant:
   `src/markdown/corpus` in Margin Docs, `simplify/.research` in Margin. Without it Margin Docs can
   never go green, because fixing a fixture breaks the round-trip test it exists for.
3. Extend it past `*.md` to `*.ts`, `*.tsx`, `*.rs`, `*.css`, `*.toml` and `*.html` so app copy is
   covered. That adds six hits in Margin and none anywhere else: user-visible strings at
   `src/export/run.ts:8` and `:36`, `src/components/Library.tsx:88` and
   `src/components/ExportPreview.tsx:185`, which are the worst place for a dash and the first to fix,
   plus one line each in `src-tauri/stubs/burn-cuda/src/lib.rs` and `stubs/cubecl-cpu/src/lib.rs`.
   It must exempt the checker's own regex (`docs-check.mjs:46-47`) and the assertion that the guide
   text is free of them (`margin-mail/src/screens/guide/guide.test.ts:104`), which necessarily
   contain the characters.

Where it lives and how it is wired. It moves into the shared repo as a bin, the way
`margin-shared-fonts` already is (`margin-mail/package.json:15` calls the bin, while
`margin/package.json:12` and `margin-docs/package.json:15` still call the script by path and should
converge on the bin form). Each app then gets a `docs` recipe in its justfile and one line in CI:
Calendar and Docs need a `- run:` added to their existing `ci.yml`, and Mail already has both. Margin
has neither a justfile nor a `ci.yml` (its workflows are `appstore.yml` and `release.yml` only), so
it needs a `docs` script in `package.json` and a small CI workflow, worth having regardless because
Margin has no typecheck or test gate at all today.

## Order of work

1. Rename `rust/margin-editor` to `margin-docs`, fix its `docs/conventions.md:3`, and rename
   `python/margin-caledar` to `margin-calendar`.
2. Create `priyanshujain/margin-mail` and push Margin Mail's work.
3. Collapse both parents into `~/Workspace/projects/margin/`: the two `package.json` shared paths,
   both lockfiles regenerated, Mail's CI paths rewritten, the second checkout added to Docs' CI, and
   the five `~/.claude/projects/` directories renamed.
4. Fix the three `conventions.md` files in place: Docs' dash wording into the other two, cut
   "Comments are rare" from all three.
5. Fix the seven em dashes in `website/README.md` and the four in Margin's user-visible strings.
6. Patch `docs-check.mjs` (code spans, skip list, source files) and copy it into the other three
   repos with the wiring above, including a first CI workflow for Margin.
7. Replace the two `CLAUDE.md` files and add one to Margin Docs and Margin Mail.
8. Write Margin's five missing docs and Margin Mail's `setup.md`, trim Mail's README, move Mail's
   `plan.md` under `docs/research/`.
9. When the shared repo exists, move `guidelines/` into it, add `guidelines/conventions.md` and
   `sync-guidelines.mjs`, and cut each app's `docs/conventions.md` down to its own rules.
10. Rename `margin-app` to `margin`, package and crate only. The bundle identifier never changes.
