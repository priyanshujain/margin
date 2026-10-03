# Docs, comments and naming across the four apps

Read: three `docs/conventions.md`, four `README.md`, every `docs/*.md`, both `CLAUDE.md`,
`shared/src/fonts.ts`, `shared/src/icons.ts`, all four `src-tauri/Cargo.toml`, all four
`package.json` and `tauri.conf.json`, `scripts/docs-check.mjs`, and the CI workflows.

## 1. The three conventions.md files

Only three exist: `margin-caledar/docs/conventions.md` (89 lines),
`margin-editor/docs/conventions.md` (103), `margin-mail/docs/conventions.md` (139). **margin, the
app all three defer to, has no conventions.md at all.** That is the first hole: the house style is
written down only in the repos that copied it, never in the repo that set it.

### What all three share, near verbatim

Each opens by disclaiming originality. Calendar line 3: "This project is a sibling to `../margin`
and follows its conventions deliberately rather than inventing new ones." Editor line 3 and mail
line 3 are the same sentence with the sibling list extended.

Five rules are word-for-word identical in all three:

- `Result<T, String>` everywhere. No `anyhow`.
- "DTOs crossing the IPC boundary live in `src-tauri/src/dto.rs` and are marked
  `#[serde(rename_all = "camelCase")]`. That file is the contract and is frozen: implementation
  modules add bodies, not fields. Its mirror is `src/ipc.ts`."
  (calendar:12, editor:13, mail:14)
- "One zustand store per domain in `src/store/`. No middleware. One selector call per field
  (`useThing((s) => s.field)`, never a destructured object), actions as inline arrow properties, and
  `set((s) => ...)` returning `{}` to no-op." (calendar:26, editor:26, mail:35)
- "Flat kebab-case class names, not BEM. State is a `data-*` attribute, never an `is-` class."
- "Transitions name explicit properties and use `var(--ease)`. Never `transition: all`."

And each closes with a `## Never` section of the same shape. Calendar:88 and mail:138: "No CSS
framework, no component library, no router, no zustand middleware, no directory trees in any
document, and no em dashes anywhere including code comments."

### Where they contradict each other

**Em dash versus en dash.** Editor:103 is the only one that bans both: "no em dashes or en dashes
anywhere, including code comments." Calendar:89 and mail:139 ban only em dashes. The user's own rule
bans both. Editor is right and the other two are stale.

**Container queries.** Calendar:67 permits exactly one, on the event block, and says "Nothing else
may reach for a container query without the same kind of reason." Mail:105 hardens it to "There is
no container query in this repository" while explicitly crediting the calendar's exception. Editor
does not mention container queries. Not a real contradiction, but three different postures.

**Where a token lives.** Calendar:46 and editor:81: "Every colour, radius and size goes through a
token in `src/styles/tokens.css`." Mail:55 redefines `tokens.css` as a seam, not a list: it imports
margin-shared's set and then `src/styles/mail.css`, and mail:86 says add to `mail.css` instead. Mail
is the only one with the three-layer tokens/primitives/screens rule (mail:51-66) and the only one
with a Kit page at `#/kit`.

**Icons.** All three keep "Inline Feather-style 24x24 stroke `d` strings passed to
`<Icon d={...} />`. There is no icon set and no registry, and there will not be one." Calendar:79 and
mail:120 add "An icon-only button always carries a `title` with its shortcut written in real
glyphs"; editor drops that line. Mail:116 is the only one that names a file (`src/ui/icons.ts`) and
the only one that acknowledges `margin-shared/icons`, which editor and calendar do not mention at
all even though `margin-shared` is a real dependency of editor.

**Comment density.** Calendar:22 and mail:31: "Comments are rare and explain why, never what. Match
the density in `lib.rs`." Editor:22 keeps the first sentence and drops the pointer. See section 5:
the density claim is false in all three.

**Errors.** Calendar allows one error enum (`google::api::ApiError`), mail allows one
(`provider::ProviderError`), editor:11 allows none. Each names its own reason. This is the right
pattern: an app-specific carve-out written next to the shared rule.

### What is genuinely app-specific and should stay that way

Editor's `## Markdown` (38-59) and `## Tests` (61-75) sections, mail's `## Places and stages`
(68-80) and `## Work packages` (129-134), calendar's `data-phone` versus `data-touch` argument
(57-70, copied verbatim into mail:94-103). Storage key prefixes differ by design: `margincal-`,
`margindocs-`, `marginmail-`.

### Broken cross-references

Calendar:3 says `../margin`, and from `python/margin-caledar` that resolves. Editor:3 says
`../margin` and `../margin-calendar`, and **neither exists**: `rust/margin` and
`rust/margin-calendar` are not on disk. Calendar:17 cites `margin/src-tauri/src/gdrive.rs:286` and
calendar:20 cites `pdf.rs:90`. Both files exist; neither line carries a comment (section 5).

## 2. READMEs

| repo | lines | verdict |
| --- | --- | --- |
| margin | 15 | compliant. Description, install, one docs link, licence. |
| margin-calendar | 15 | compliant, but the last seven lines are one dense paragraph of six links. |
| margin-docs | 14 | compliant and the cleanest of the four. |
| margin-mail | 20 | over. Lines 3 to 10 are an eight-line pitch that belongs in `docs/design.md`. |

Mail is the offender. Its opening paragraph restates the product ("New senders wait at the door
until you let them in; people, newsletters and receipts live in three separate boxes") which is
already `docs/design.md:31-73`. Cutting it to two sentences puts it at 13 lines.

Four other READMEs exist inside margin and are not project descriptions:
`margin/website/README.md` (48 lines, 7 em dashes), `margin/appstore/screenshots/README.md` (51),
`margin/simplify/README.md` (26), `margin/simplify/guidelines/README.md` (31).
`margin/shared/README.md` is 15 lines and compliant. The website one is the worst artefact in the
suite: bold-marked feature bullets, em dashes throughout, a "Built with" line. It reads like a
different author.

`margin/shared/README.md:3` and `shared/package.json:7` both say the package is for "Margin and
Margin Docs". Margin Mail has depended on it since `package.json:24`. The description is stale.

## 3. The docs set

- **margin**: `docs/publishing.md` only (229 lines). No architecture, no design, no conventions, no
  setup, no release. The originating app is the least documented.
- **margin-calendar**: `architecture.md` (145), `conventions.md` (89), `design.md` (152),
  `mobile.md` (304), `release.md` (105), `setup.md` (55). This is the set that got copied.
- **margin-docs**: `architecture.md` (545), `conventions.md` (103), `design.md` (131),
  `release.md` (117), `setup.md` (29). Calendar's set minus `mobile.md`, macOS only.
- **margin-mail**: `architecture.md` (352), `conventions.md` (139), `design.md` (153),
  `release.md` (119), plus `features.md` (475), `ui.md` (339), `settings.md` (201), `plan.md` (193),
  `keyboard.md` (129), `help.md` (74), `mockups/`, `research/`. **No `setup.md`**, which is the one
  file a new machine needs, and mail is the app that requires a Google OAuth client.

Three shapes are stable across the set. `architecture.md` always opens with the same stack sentence:
"Tauri 2, React 19, Vite, TypeScript and zustand on the front, Rust behind" (calendar:3, editor:3,
mail:3), then "The split is strict" and what each side owns. `design.md` is always product decisions
argued as prose under "why" headings, and always ends with `## Visual language`. `release.md` always
starts `## Installing locally` then `## Cutting a release` and ends `## Updates`.

### Proposed canonical set

Five files, every app, same names, same opening move:

`architecture.md`, `conventions.md`, `design.md`, `setup.md`, `release.md`.

Then only what the product actually has: `features.md`, `ui.md`, `keyboard.md`, `settings.md`,
`help.md`, `mobile.md`, `publishing.md`, `plan.md`, `research/`, `mockups/`. Never a numeric prefix.

Concretely: margin needs all five written and should keep `publishing.md`; mail needs `setup.md`;
mail's `plan.md` is a milestone tracker and will go stale, so it belongs in the issue tracker or
under `research/`.

## 4. The comment voice

### What a comment is for here

A comment records a decision and the alternative that was rejected, so that a competent person does
not undo it by accident. It never says what the line below does.

The shape is consistent enough to be a template. A file-head block states what the file is in one
line, then a blank comment line, then one paragraph per decision. Each paragraph names the thing
chosen, then the thing not chosen, then the concrete failure the wrong choice produces. Sentences
are long, declarative, no hedging, no first person, no "note that". Length is one to seven lines per
paragraph; a file head runs 3 to 17 lines. Doc comments on exported items are one sentence and often
end on the consumer ("which is what a Typst preamble names a face by").

The tell is that almost every comment contains a causal clause: "because", "so that", "which is
why", "rather than", "or a ... would".

### Four exemplars, in full

`margin/shared/src/fonts.ts:1-7`:

    // The faces both apps offer, and the two slots they set them into.
    //
    // This lives in one place because the two apps have to agree about it. A face named here is a
    // `@font-face` in css/fonts.css, a file in fonts/, and a family a Typst preamble names on the way
    // to a PDF, and those four lists going out of step with each other is a document that renders in
    // one app and falls back to Georgia in the other. There is no way to keep four lists in two repos
    // honest by hand, so there is one list.

`margin/shared/src/icons.ts:3-6`:

    // Here because the two apps kept drifting. Each had its own idea of what a search or a moon looked
    // like, they were adjusted independently, and the result was two products from the same hand that
    // did not look related. A path is a design decision, not a detail, and the fix for two copies of a
    // decision is one copy.

`margin-mail/src-tauri/Cargo.toml:61-63`:

    # Refresh tokens, the backup key and every uploaded journal segment are sealed with
    # XChaCha20-Poly1305. There is no `keyring` here on purpose: it has no Android backend at all, and
    # on macOS it ties the item to the code signature, so every rebuild re-prompts.

`margin-editor/src-tauri/Cargo.toml:54-56`:

    # Pinned exactly: the [patch] stubs at the foot of this file are tied to this version's burn/cubecl
    # graph. A minor bump could silently invalidate a patch ("unused"), and the whole CUDA and LLVM
    # subtree those stubs remove would come back. Bump deliberately and re-audit the stubs.

`margin/shared/src/icons.ts:29-37` is the purest case, because it justifies an SVG path: two
letterforms rather than one on a rule, because one letter over a full-width line is the underline
button in every editor, and because the neighbouring `SPELLING` glyph is also built on a capital A,
so the distinguishing feature has to be the bowl versus the tick, which survives at 16px.

### Violations, in both directions

**Undocumented decisions.** The clearest cases are the exact decisions two sibling repos cite by
file and line:

- `margin/src-tauri/src/gdrive.rs` is 953 lines with **zero comments**. Calendar's conventions.md:17
  points at `gdrive.rs:286` as the origin of `read_json` and explains why the body goes to a `String`
  first. The reason is written in the calendar's docs and in mail's docs and never in the file.
- `margin/src-tauri/src/pdf.rs` is 129 lines with zero comments. Calendar:20 and mail:23 both cite
  `#[tauri::command(async)]` on a synchronous fn at `pdf.rs:90` as the trick for getting off the main
  thread. `compile_pdf` at line 90 carries no comment saying so.
- Other zero-comment files over 200 lines in margin: `src/components/EditorView.tsx` (560),
  `src/import/epub.ts` (534), `src/export/typst.ts` (421), `src/components/Sidebar.tsx` (310),
  `src-tauri/src/proofing.rs` (275), `src/model/book.ts` (260), `src-tauri/src/lib.rs` (256),
  `src/editor/search.ts` (233), `src/components/Dock.tsx` (223).

**Narration that should go.**

- `margin/src/components/ExportPreview.tsx:320`: `// render cancelled or page failed; keep the
  previous canvas`. Lowercase, no full stop, and the second clause narrates the line below.
- `margin/src-tauri/Cargo.toml:9`: `# See more keys and their definitions at
  https://doc.rust-lang.org/cargo/reference/manifest.html`, and lines 12-14, the `_lib` suffix
  paragraph. Both are `cargo new` boilerplate. The three sibling Cargo.toml files deleted them; margin
  did not.
- `// Prevents additional console window on Windows in release, DO NOT REMOVE!!` is
  `src-tauri/src/main.rs:1` in all four repos. Tauri scaffold text, shouting, and none of these apps
  ships on Windows.
- Four `// eslint-disable-next-line react-hooks/exhaustive-deps` in margin
  (`src/components/FindBar.tsx:44`, `src/editor/FloatingToolbar.tsx:69`, `src/editor/Editor.tsx:113`
  and `:122`). No repo has eslint configured, in package.json or on disk. Dead directives.

Beyond these, a sweep for narration patterns across all four repos found almost nothing. Every hit
on "comment starts with a verb" or "comment starts lowercase" turned out to be a wrapped
continuation line of a real reason. The voice is being held.

## 5. The CLAUDE.md tension, resolved

`margin/CLAUDE.md:9` and `margin-caledar/CLAUDE.md:9` are byte-identical: "Avoid excessive comments.
Only comment when absolutely necessary. Code should be readable and not require comments to
understand it." margin-docs and margin-mail have no CLAUDE.md.

The code says otherwise. Comment lines as a fraction of source in `src/`, `src-tauri/src/` and
`shared/src/`:

| repo | source lines | comment lines | share |
| --- | --- | --- | --- |
| margin | 10,377 | 125 | 1.2% |
| margin-calendar | 20,211 | 2,208 | 10.9% |
| margin-docs | 43,973 | 10,634 | 24.2% |
| margin-mail | 70,737 | 9,921 | 14.0% |

margin obeys the rule as written. The three younger apps ignore it by a factor of ten to twenty, and
they are the disciplined ones. The rule was true of a repo that had no siblings; it stopped being
true the moment a decision had to survive being copied into another repo.

The operating rule, read off the code: **a comment never says what, and always says why.** The test
already exists in `simplify/guidelines/code-style.md:30`: "delete the comment and ask whether a
competent person would make the same mistake twice. If yes, keep it. If it just narrates the line
below, cut it." Under that test margin is not compliant by being sparse; it is under-commented, and
`gdrive.rs` is the proof.

The two CLAUDE.md files should be corrected or deleted. As written they are a live instruction to
strip the best thing in this codebase.

## 6. Typographic compliance

Across `*.md`, `*.ts`, `*.tsx`, `*.rs`, `*.css`, `*.toml`, `*.html`, `*.js`, `*.mjs`, excluding
`node_modules`, `dist`, `target`, `.git`, `target-mas`, `.playwright-mcp` and `gen`:

| repo | em dashes | en dashes |
| --- | --- | --- |
| margin | 37 | 5 |
| margin-calendar | 0 | 0 |
| margin-docs | 7 | 0 |
| margin-mail | 3 | 2 |

margin-calendar is perfectly clean. margin holds 42 of the 54 in the suite.

Worst files:

- `margin/simplify/.research/memories-raw.md`, 19. A dump of old memory files, so arguably input
  rather than committed prose, but it is in the repo.
- `margin/website/README.md`, 7, all in body copy (lines 3, 21, 22, 23, 25, 37, 41).
- `margin-docs/src/markdown/corpus/real/margin-website-readme.md`, 7. A copy of the file above, kept
  as a serializer test fixture. Fixing the website README without fixing the fixture will not help,
  and fixing the fixture may break a round-trip test.
- `margin/src/export/run.ts:8` and `:36`, `margin/src/components/Library.tsx:88`,
  `margin/src/components/ExportPreview.tsx:185`. These four are **user-visible app copy**, which is
  the worst place for it: the string at `run.ts:8` puts one between "desktop app only" and "open the
  window from".
- `margin/src-tauri/stubs/burn-cuda/src/lib.rs:1` and `stubs/cubecl-cpu/src/lib.rs:1`, one each.
- `margin/simplify/guidelines/prose-and-docs.md:8` is the rule itself quoting both characters. Fine.

margin-mail's three are all legitimate: `scripts/docs-check.mjs:46-47` is the regex that enforces the
rule, and `src/screens/guide/guide.test.ts:104` asserts the guide text is free of them.

### The gate already exists, in one repo

`margin-mail/scripts/docs-check.mjs` is a 72-line node script wired to `just docs`
(`margin-mail/justfile:33-34`). It walks every markdown file and reports em dashes, en dashes,
directory trees and dead relative links. Its own header, lines 2-11, is a model comment. **No other
repo has it**, and margin, which has 42 offences, is the repo that most needs it. Copying that one
file into the other three, and extending it past `*.md` to source files so app copy is covered, is
the single highest-value action in this report.

## 7. Directory trees

None. A search for box-drawing runs and for the ASCII form across markdown, TypeScript, Rust, JSON
and text in all four repos returned nothing. The rule is being kept without a gate in three of the
four repos, which is worth noting: the risk is a future file, not an existing one.

## 8. Naming

| | margin | calendar | docs | mail |
| --- | --- | --- | --- | --- |
| directory | `python/margin` | `python/margin-caledar` | `rust/margin-editor` | `rust/margin-mail` |
| package.json name | `margin-app` | `margin-calendar` | `margin-docs` | `margin-mail` |
| Cargo package | `margin-app` | `margin-calendar` | `margin-docs` | `margin-mail` |
| Cargo lib | `margin_app_lib` | `margin_calendar_lib` | `margin_docs_lib` | `margin_mail_lib` |
| bundle id | `studio.margin.app` | `studio.margin.calendar` | `studio.margin.docs` | `studio.margin.mail` |
| git remote | `priyanshujain/margin` | `priyanshujain/margin-calendar` | `priyanshujain/margin-docs` | **none** |
| productName | `Margin` | `Margin Calendar` | `Margin Docs` | `Margin Mail` |
| html title | `margin` | `Margin Calendar` | `Margin Docs` | `Margin Mail` |
| README h1 | `margin` | `Margin Calendar` | `Margin Docs` | `Margin Mail` |
| storage prefix | (none stated) | `margincal-` | `margindocs-` | `marginmail-` |
| licence | FSL-1.1-MIT | MIT | MIT | FSL-1.1-MIT |

Disagreements, worst first:

1. **`python/margin-caledar` is a typo.** Confirmed. Everything inside it says `margin-calendar`,
   including the remote and the Nix flake output (`ci.yml:81`, `nix build .#margin-calendar`).
2. **`rust/margin-editor` versus `margin-docs`.** Confirmed. The directory is the only place the
   word "editor" appears as a name; package, crate, bundle id, product name and remote all say docs.
3. **`python/` and `rust/` parents are wrong for all four.** All four are Tauri apps with a React
   front end and a Rust backend. None is a Python project. This is not cosmetic:
   `margin-docs/package.json:33` and `margin-mail/package.json:24` both carry
   `"margin-shared": "file:../../python/margin/shared"`, and `margin-mail/.github/workflows/ci.yml`
   checks the two repos out into `rust/margin-mail` and `python/margin` (lines 25 and 30) precisely to
   reproduce that path. The word "python" is baked into a GitHub runner's filesystem layout.
4. **margin-docs CI is failing on exactly this.** The latest run's failure is
   `ENOENT: no such file or directory, scandir '/Users/runner/work/python/margin/shared'`. Editor's
   `ci.yml` runs `pnpm install --frozen-lockfile` after a single checkout, so the relative path has
   nothing to resolve to. Mail solved it with a second checkout; docs never did. Five of the last six
   runs failed.
5. **margin-mail has no git remote.** One local commit, `088ec9c`. Everything else in the suite is on
   GitHub. Whatever name the repo gets is still an open choice, which makes this the cheapest moment
   to fix the pattern.
6. **`margin-app` versus `Margin`.** margin is the only app whose package and crate name is not its
   product name lowercased and hyphenated. `margin-app` also breaks the bundle id pattern:
   `studio.margin.app` reads as a namespace with a placeholder in it.
7. **`margin/index.html` title is lowercase `margin`** while `productName` is `Margin`. The README h1
   is lowercase too. The other three are consistent title case.
8. **Licences split two and two**, and neither README in the MIT pair says so. margin and mail are
   FSL, calendar and docs are MIT, and nothing explains the split.
9. **Editor's conventions.md:3 points at `../margin` and `../margin-calendar`**, neither of which
   exists relative to `rust/margin-editor`. The paths only make sense if all four sit as siblings,
   which is what the rename below produces.
10. `margin-docs/src/markdown/corpus/real/` holds copies of the calendar's and the editor's own
    `conventions.md` as test fixtures. Two of the three canonical convention documents exist twice in
    the suite and will drift.

## 9. Rename proposal, with cost

Target layout: one parent, `~/Workspace/projects/margin/`, holding `margin`, `margin-calendar`,
`margin-docs`, `margin-mail` as siblings.

**A. `margin-caledar` to `margin-calendar`.** Cheapest and unambiguous.
Cost: `mv` the directory. No `package.json` anywhere references it. Nothing on GitHub changes; the
remote is already correct. Only local shell history and any editor workspace file break.

**B. `margin-editor` to `margin-docs`.** Also cheap.
Cost: `mv` the directory. The `file:../../python/margin/shared` path is unaffected because the depth
does not change. `docs/conventions.md:3` should be corrected in the same edit. No remote change: the
remote is already `margin-docs`.

**C. Collapse `python/` and `rust/` into one `margin/` parent.** The expensive one, and the one that
pays.
Cost, exhaustively:
- `margin-docs/package.json:33` and `margin-mail/package.json:24`: `file:../../python/margin/shared`
  becomes `file:../margin/shared`. Both lockfiles need regenerating.
- `margin-mail/.github/workflows/ci.yml:21,25,29,30,57,61`: the checkout paths `rust/margin-mail` and
  `python/margin` become `margin-mail` and `margin`, and the `working-directory` lines follow.
- `margin-docs/.github/workflows/ci.yml`: needs the second checkout added, which it is currently
  missing. This fixes the failing build rather than costing anything.
- Local shell history, editor workspaces, and any absolute path in a memory file.
- Nothing on GitHub changes. No remote, no clone URL, no release artefact, no bundle id.

**D. `margin-app` to `margin`, and `studio.margin.app` to `studio.margin.writer` or similar.**
Recommend doing the crate and package rename and **not** the bundle id.
Cost of the package and crate rename: `package.json:2`, `src-tauri/Cargo.toml:2`, `Cargo.lock`, the
lib name `margin_app_lib` and every `use margin_app_lib::` in `src-tauri/src/main.rs`, plus any
workflow that names the binary.
Cost of the bundle id rename: **do not**. Changing `identifier` on a shipped macOS app orphans the
application support directory, breaks the Homebrew cask, breaks the updater's signature check, and
makes the Mac App Store record a different product. The inconsistency is not worth that. Write one
line in `docs/conventions.md` saying `studio.margin.app` is frozen and why.

**E. Give margin-mail a remote.** `priyanshujain/margin-mail`, matching the other three. Free now,
and the naming stays consistent by default.

Order: B, A, E, C, D. B and A are free. C is best done immediately after, while mail has no CI
history to invalidate.

## 10. The proposed house style

### Docs

A README is the project description. Under 15 lines: one paragraph on what it is, one line on how to
install, one paragraph of links into `docs/`, one line on the licence. margin-docs' README is the
model. Mail's is eight lines over and those eight lines are already in `docs/design.md`.

`docs/` holds `architecture.md`, `conventions.md`, `design.md`, `setup.md`, `release.md` in every
app, plus whatever the product genuinely has. No numeric prefixes. No file that is a status report.

`architecture.md` opens with the stack sentence and "The split is strict", then what each side owns,
then one section per hard part, then `## Order of work`. `design.md` argues product decisions in
prose under "why" headings and ends with `## Visual language`. `release.md` runs `## Installing
locally`, `## Cutting a release`, `## What the build needs`, `## Updates`.

`conventions.md` exists in every app including margin, and is split: the shared rules (the five
identical ones, verbatim) and the app's own. When an app carves out an exception, the carve-out names
the reason in the same sentence, the way mail:9 names `provider::ProviderError` and the four cases it
has to branch on. Both em dashes and en dashes are banned, taking editor's wording over the other
two.

Prose a colleague would write. No em dash, no en dash, no directory tree, ever, and the rule holds
inside code fences and app copy as well as in body text.

### Comments

A comment records a decision and the alternative that was rejected. It never says what the line below
does; if you want to, rename something instead.

Shape: a one-line statement of what the file is, a blank comment line, then one paragraph per
decision. Each paragraph names what was chosen, what was not, and the concrete failure the other
choice produces. One to seven lines per paragraph. Declarative, third person, no hedging. Doc
comments on exported items are one sentence.

The test, already written at `simplify/guidelines/code-style.md:30`: delete it and ask whether a
competent person would make the same mistake twice.

Two consequences worth stating plainly, because the current text says the opposite. The instruction
in `margin/CLAUDE.md:9` and `margin-caledar/CLAUDE.md:9` is wrong and should be replaced with the
rule above. And "comments are rare" in all three conventions.md files is false and should be cut: the
suite averages one comment line in seven, and the three densest repos are the three best ones.

### Enforcement

Copy `margin-mail/scripts/docs-check.mjs` into the other three repos, wire it to `just docs` and to
CI, and extend it beyond `*.md` to `*.ts`, `*.tsx`, `*.rs` and `*.css` so app copy is covered. That
one file catches every offence in section 6, plus the dead links, plus any future tree. Fix margin's
42 dashes first, starting with the four in user-visible strings.
