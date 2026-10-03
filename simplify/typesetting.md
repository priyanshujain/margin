# Typesetting, fonts and proofing

The plan for `margin-typeset`, `margin-mac` and the font packages, from the measurements in
[.research/typesetting-and-text.md](.research/typesetting-and-text.md), with names from
[repo-layout.md](repo-layout.md).

## One file that was copied, and only one copy was maintained

The problem is not that Margin and Margin Docs both compile Typst. It is that Docs' Typst stack is
Margin's, forked when Docs was scaffolded, and every fix since has landed on one side. Docs is ahead
on all six Rust pairs and four of the six frontend pairs, and in three places Margin does not merely
lag: it ships a defect Docs found, wrote a paragraph about, and fixed. The shared crate is the only
mechanism that would have carried those three fixes back.

Overlap below is "identical in order": an LCS over code lines, comments and blanks stripped, as a
percentage of the smaller file. Numbers from the audit.

| Pair | Margin | Margin Docs | Identical | % of smaller | Ahead |
| --- | --- | --- | --- | --- | --- |
| `src-tauri/src/pdf.rs` | 129 (116 code) | 455 (275) | 37 | 31% | Docs |
| `src-tauri/src/fonts.rs` | 47 (43) | 182 (113) | 17 | 39% | Docs |
| `src-tauri/src/macspell.rs` | 73 (68) | 162 (82) | 49 | 72% | Docs |
| `src-tauri/src/writingtools.rs` | 81 (68) | 146 (83) | 42 | 61% | Docs |
| `proofing.rs` vs `grammar.rs` | 275 (239) | 128 (58) | 33 | 56% | Docs |
| `stubs/*` (4 files) | 37 | 37 | all | 100% | neither |
| `src/export/typst.ts` | 421 (335) | 808 (486) | 9 | 2% | neither |
| `src/components/ExportPreview.tsx` | 336 (307) | 448 (342) | 200 | 65% | Docs |
| `src/components/ProofPopover.tsx` | 76 (68) | 292 (184) | 29 | 42% | Docs |
| `src/editor/proofing.ts` | 139 | 642 (344) | 17 | 13% | Docs |
| `src/editor/search.ts` | 233 (217) | 264 (235) | 206 | 94% | neither |
| `src/editor/paste.ts` | (70) | (163) | 51 | 72% | Docs |
| `src/escape.ts` | 36 | 36 | 36 | 100% | neither |

What the drift has cost, all of it paid by Margin, all of it fixed in Docs already:

- The three bugs below: every heading and bold run in a Margin PDF exports at weight 400, `\b` and
  `\f` and `\u001b` reach the page as visible backslash text, and a system family that is a `.ttc` is
  read and shipped to the compiler four times.
- An inverted Harper span panics: `chars[start..end]` at `margin/src-tauri/src/proofing.rs:171-173`
  clamps start and end independently, where Docs clamps end first then start against end
  (`grammar.rs:84-85`). An `NSRange` carrying `NSNotFound` wraps or panics the same way, because
  `margin/src-tauri/src/macspell.rs:53` adds before it clamps and Docs uses `saturating_add` at
  `macspell.rs:107`.
- Harper's "French spaces" lints draw an underline over two spaces that cannot be clicked; Docs
  drops whitespace-only lints at `grammar.rs:96-98`.
- Roughly 1.5M of pdf.js sits in Margin's main bundle whether or not the export panel is opened,
  because `ExportPreview.tsx:18` sets `GlobalWorkerOptions.workerSrc` at module scope. Docs
  dynamic-imports it in `margin-editor/src/pdfjs.ts`, 27 lines.
- Margin names no monospace and no math family, so a code block gets whatever Typst defaults to and
  a formula on a machine without New Computer Modern Math is a hard compile error, not a warning.
  Docs' `fonts.rs:19-42` and `pdf.rs:165-194` exist for exactly that.

Both trees resolve the same 960-package graph to the same versions by hand, twice: harper-core 2.5.0,
typst 0.14.2, fontdb 0.23.0, burn-cuda 0.19.1, cubecl-cpu 0.8.1.

## The three bugs

All three verified in this pass, against the source and the dependency sources.

**1. `JSON.stringify` is not a Typst escaper. Confirmed.**
`margin/src/export/typst.ts:36-38` is `function str(value) { return JSON.stringify(value) }`. A
Typst string literal resolves exactly `\\`, `\"`, `\n`, `\r`, `\t` and `\u{...}`; every other
escape falls through the `_ => out.push_str(s.from(start))` arm at
`typst-syntax-0.14.2/src/ast.rs:1250-1270` and is copied to the page as literal text. I checked what
`JSON.stringify` emits in node: `\b` for U+0008, `\f` for U+000C, and `\uXXXX` (no braces) for every
other C0 control and for a lone surrogate. None of those five forms is Typst syntax, so each lands
in the PDF as the backslash sequence itself: an escape character between two letters arrives as the
eight characters `a\u001bb`, and there is no compile error. The busiest call site is `typst.ts:52`,
`#raw(${str(node.text ?? "")})`, every inline code span in the book; then the href at `:60`, the
title and author at `:136` and `:374`, and figure paths at `:83` and `:89`.

One correction to the audit: `JSON.stringify` does **not** escape U+2028 or U+2029, it passes them
through raw. Separate and smaller, and `esc()` at `typst.ts:26-28` already folds them to a space on
the markup path. Fixed by the extraction only if the escaper is shared where it runs, which is
TypeScript; see section 4.

**2. Variable fonts export at one weight. Confirmed.**
`margin/src-tauri/src/pdf.rs:9-20` are eleven `include_bytes!` of `public/fonts/*-VF.ttf`, and
`pdf.rs:57-62` pushes the Literata and Hanken variable files into the engine. Typst has no variable
axis: it lays out at the default instance and warns that it did. So a Margin PDF has no typographic
hierarchy at all, headings at the same weight as the body. Docs cut nine static instances into
`margin-editor/src-tauri/fonts/` (1.6M plus `PROVENANCE.md`) and loads those at `pdf.rs:32-42`, with
the reason at `pdf.rs:24-31`, and it documents the residual caveat for the other four families at
`pdf.rs:44-51`: EB Garamond, Lora, Source Serif and Fraunces are still loaded as variable files and
still export flat, which Docs judged not worth eight more cuts. Margin has never noticed either half.
This is the one bug the extraction fixes on its own, because the fix is an asset the crate carries.

**3. Faces deduplicated by face id, not by file. Confirmed.**
`margin/src-tauri/src/fonts.rs:30` and `:39`: `seen.insert(id)` over `fontdb::ID`, which is per face.
`db.with_face_data` hands back the whole file, so a `.ttc` holding regular, italic, bold and bold
italic has four ids, passes the `seen` check four times, and the same multi-megabyte collection is
copied into the font list four times. On macOS almost every system family is a `.ttc`. Docs keys on
the source path (`margin-editor/src-tauri/src/fonts.rs:55-61`, used at `:88`) and reads it once.
Fixed by the extraction: there is one loader and it is Docs'.

**The `installed()` guard: the audit's correction is right. Confirmed.**
`margin-editor/src-tauri/src/fonts.rs:63-67` is checked at `:88` right after a successful `db.query`,
and the comment at `:84-86` says fontdb returns a closest match rather than nothing. It does not.
`Database::query` at `fontdb-0.23.0/src/lib.rs:661-678` builds its candidate list with
`face.families.iter().any(|family| family.0 == name)`, exact string equality, and returns `None` when
the list is empty. The guard cannot fire, and since it compares case-insensitively it is strictly
weaker than the filter that already ran. It is live and correct at `:139`, asked about `db.faces()`
directly. Keep the function, delete the call at `:88` and the comment. Same code in fontdb 0.24
(`lib.rs:663-681`), which matters because Margin Mail is on 0.24.

## The harper patch trap

The strongest argument for a shared crate here is four lines that fail silently. Both apps pin
`harper-core = { version = "=2.5.0", features = ["concurrent"] }`
(`margin/src-tauri/Cargo.toml:38`, `margin-editor/src-tauri/Cargo.toml:57`). Harper pulls burn,
which declares optional, disabled `burn-cuda` and `cubecl-cpu` backends. Both are off, and cargo
resolves them anyway, so both repos replace them with do-nothing crates:

    [patch.crates-io]
    burn-cuda = { path = "stubs/burn-cuda" }
    cubecl-cpu = { path = "stubs/cubecl-cpu" }

`margin/src-tauri/Cargo.toml:70-72` and `margin-editor/src-tauri/Cargo.toml:98-100`. The four stub
files are code-identical across the two repos, only the prose comments differ, and both repos also
carry `[profile.dev.package."*"] opt-level = 3` (Margin `:76-79`, Docs `:105-108`) because Harper's
burn-ndarray POS tagger is roughly ten times slower unoptimized.

The stub versions, burn-cuda 0.19.1 and cubecl-cpu 0.8.1, and the feature lists that have to satisfy
burn's `burn-cuda?/...` references (`std`, `doc`, `fusion`, `autotune`, `autotune-checks`) are facts
about harper-core 2.5.0's exact transitive graph. Bump harper and they may not match. Cargo treats a
`[patch]` entry that matches nothing as a **warning**, not an error, so what silently returns is the
real `cubecl-cpu`: several hundred crates and an LLVM/MLIR toolchain back in the build. Nothing
fails; the symptom is a build that got slow, noticed weeks later. No test in either repo asserts the
patch still applies, and no justfile checks.

The check that should exist is cheap: when a patch applies, the package's entry in `Cargo.lock` has
no `source` and no `checksum`. Both lockfiles show exactly that today
(`margin/src-tauri/Cargo.lock:659-661` for burn-cuda, `:1650-1651` for cubecl-cpu; Docs at
`:1596-1597`), where fontdb at `:2699-2702` carries both. So the test is a lockfile parse, no
network and no cargo invocation:

```rust
for name in ["burn-cuda", "cubecl-cpu"] {
    let entry = lock_entry(include_str!("../../Cargo.lock"), name);
    assert!(entry.source.is_none(), "{name} came from the registry: the [patch] went stale");
}
```

One constraint decides where this lives: **cargo only honours `[patch]` from the top-level
workspace manifest and ignores it in dependencies**, and `[profile]` is the same. So a shared crate
cannot carry the patch block, and copying stub directories into every app defeats the point. The
workable shape is a git-sourced patch, which needs no registry:

    [patch.crates-io]
    burn-cuda = { git = "https://github.com/priyanshujain/margin-shared", tag = "v0.3.0" }
    cubecl-cpu = { git = "https://github.com/priyanshujain/margin-shared", tag = "v0.3.0" }

with packages literally named `burn-cuda` and `cubecl-cpu` in the shared repo under `crates/stubs/`.
Each app keeps four lines it can copy and cannot get wrong, one repo owns the versions and feature
lists, and the lockfile test ships as a snippet each app's gate runs.

Repo-layout.md has no crate for Harper and needs one. The audit called it `margin-grammar` and that
name should stand: it owns `build_harper` (Margin `proofing.rs:98-103`, Docs `grammar.rs:41-47`, the
same five lines including `set_rule_enabled("SpellCheck", false)`, because the system checker does
spelling better), `collect_grammar` with Docs' clamp order and whitespace-lint drop, `GrammarIssue`,
and the stub packages and lockfile test above.

## `margin-typeset`

Owns everything that knows about the Typst engine and nothing that knows what document is being
written.

- The engine and its world: font bytes in, source and images in, PDF bytes and warnings out.
- The font resolver: the fontdb database, the installed-family list, the four-style loader keyed on
  source path, the fallback collection, and the bundled faces including the nine cuts (section 5).
- Diagnostic formatting: `margin/src-tauri/src/pdf.rs:113-129` and
  `margin-editor/src-tauri/src/pdf.rs:439-455` are the same sixteen lines, character for character,
  differing only in the name.
- Warning aggregation with `kind` and `count`, Docs' `note` at `pdf.rs:201-215` and `PdfWarning` at
  `dto.rs:216-222`, so forty unparseable formulas are one toast with a number on it.
- Image handling: inline base64, the root-guarded filesystem read, and the one-pixel placeholder per
  format so an unreadable image is a gap and not a failed export (Docs `pdf.rs:122-163`).
- PDF output options, the `&Default::default()` both apps pass to `typst_pdf::pdf` today, so metadata
  and PDF/A get decided once; and error mapping, `TypstAsLibError` and `&[SourceDiagnostic]` to one
  `String` or a `Vec<Warning>`.

Does not own each app's preamble or converter: 2% overlap, 9 lines of 335, and those nine are the
image extension sniffer. Margin writes a book (trim sizes at `typst.ts:16-21`, title page, chapter
and part openers, a TOC querying `<chap>` metadata at `:130-239`); Docs writes an A4 document
(callouts, task lists, tables, code surfaces, mermaid, mitex math at `:206-327`).

```rust
pub struct Faces { pub bundled: Vec<String>, pub system: Vec<String> }
pub struct Fallbacks { pub fonts: Vec<Vec<u8>>, pub monospace: Vec<String>, pub math: Vec<String> }
pub struct ImageInput { pub path: String, pub data: Option<String> }
pub struct Warning { pub kind: &'static str, pub message: String, pub count: u32 }

pub struct Job<'a> {
    pub source: String,
    pub images: &'a [ImageInput],
    /// Directories a file-backed image may be read from. Empty means inline bytes only.
    pub roots: &'a [String],
    pub faces: Faces,
    pub options: typst_pdf::PdfOptions<'a>,
}

/// `Err` is the formatted diagnostics; warnings never fail an export.
pub fn compile(job: Job<'_>) -> Result<(Vec<u8>, Vec<Warning>), String>;

/// Typst source naming the monospace and math families this machine has, to concatenate ahead of
/// the caller's own preamble. Only what was found, because Typst warns once per family it missed.
pub fn fallback_preamble(fallbacks: &Fallbacks) -> String;

/// A Typst string literal: `\\ \" \n \r \t \u{...}` and nothing else.
pub fn escape_string(value: &str) -> String;

pub mod fonts {
    pub fn system_families() -> Vec<String>;          // the eight-line list, three copies today
    pub fn faces_for(family: &str) -> Vec<Vec<u8>>;   // four styles, deduplicated by source path
    pub fn fallbacks() -> Fallbacks;
    pub fn bundled(id: &str) -> &'static [&'static [u8]];
}
```

The `pdf` feature is on by default and gates typst, typst-as-lib and typst-pdf; `fonts` is always
compiled. Margin Mail takes `default-features = false` and gets eight lines of fontdb without
dragging the Typst compiler into a mail client.

On the escaper: `escape_string` has a real Rust consumer, `font_list` at Docs' `pdf.rs:165-169`,
which writes `format!("\"{f}\"")` with no escaping at all, safe only because the two family lists are
consts. But the converters build their Typst source in TypeScript, so the escaper that fixes bug 1
has to be TypeScript too. Both sides ship the same rule against one table of test vectors in the
shared repo (the C0 controls, a lone surrogate, U+2028, a quote, a backslash), so they cannot drift
the way the two `str` implementations already did.

## Fonts: one catalogue, four lists, no Rust half

`margin/shared/src/fonts.ts` is 233 lines and is already the single source of truth for the six
bundled families: the `FontRef` encoding, the pairings, `fontsUsed`. All three UI apps import it,
including Margin Mail at `src/screens/Settings.tsx:12`. Its own header states the problem it exists
to solve: a face named there is a `@font-face` in `shared/css/fonts.css`, a file in `shared/fonts/`,
and a family a Typst preamble names, and four lists in two repos cannot be kept honest by hand.

Three of the four are wired together by `shared/bin/sync-fonts.mjs`, which every app already runs as
`fonts:sync` and `fonts:check`. The fourth never got done: the Typst family names are re-declared as
`include_bytes!` lists in `margin/src-tauri/src/pdf.rs:9-35` and
`margin-editor/src-tauri/src/pdf.rs:32-88`, and nothing checks either against the catalogue. The
installed-family list is written three times, `margin/src-tauri/src/fonts.rs:11-18`,
`margin-editor/src-tauri/src/fonts.rs:159-168` (byte-identical) and
`margin-mail/src-tauri/src/settings.rs:179-189` (same algorithm, `system_db()` inlined). Mail is on
fontdb 0.24 and the others on 0.23; the two crates' public signatures diff clean, so one pin at 0.24
is mechanical.

The bytes are on disk four times at 6.8M each, plus 1.6M of cuts: 28.8M of repo for six families.

- `@margin/fonts` owns the catalogue in TypeScript, the `@font-face` block, the variable binaries,
  their OFL files and `sync-fonts`. The catalogue stays the source of truth: it is what a human edits.
- `margin-typeset` owns the Rust half: the nine static instances and their `PROVENANCE.md`, the
  variable files for the other four families, the id-to-bytes mapping and the Typst family names.
- The join is generated, not written. `sync-fonts --rust` emits the crate's catalogue module from
  `fonts.ts`, and `fonts:check` fails when it is stale, exactly as it already fails when
  `public/fonts` is stale. Four lists become one list and three generated views, and the failure
  mode moves from "renders in one app and falls back to Georgia in the other" to a red CI job.

The static instances go in the crate and not in `@margin/fonts` because they are useless to a
browser, and their provenance note is the regeneration instruction, which belongs beside the files.

## `margin-mac`

One `cfg(target_os = "macos")` boundary for the three AppKit and Foundation integrations.

**Writing Tools.** The AppKit menu walk is the cleanest extraction in the audit: it takes a
`MainThreadMarker` and a title and knows nothing about either app. `submenu_named` and `edit_menu`
are 21 lines byte-identical, `margin/src-tauri/src/writingtools.rs:10-30` against
`margin-editor/src-tauri/src/writingtools.rs:23-43`, and `writing_tools_menu` is the same one-liner
(Margin `:32-34`, Docs `:45-47`, differing by `pub`). Take Docs' availability probe with it, the
`SUBMENU_SEEN` atomic at `:13-14` and `:92-105` and `writing_available` at `:106-120`, so an app can
say the machine has no Apple Intelligence instead of offering a dead button. Docs' `perform` returns
three distinct messages (`:58-73`); Margin's returns `()` and no-ops when the row is missing
(`:51-61`).

The key equivalents stay in the apps and are not a drift to resolve. Margin puts Shift+Option+F and
Shift+Option+R on Apple's own Proofread and Rewrite rows (`writingtools.rs:8, 36-49`). Docs refuses,
and `writingtools.rs:85-91` says why: AppKit performs a key equivalent by firing the menu item
directly, so a chord on the system's row reaches Writing Tools without passing the selection guard
in `src/editor/writing.ts`. Margin has no such guard so it is not wrong today, but the crate should
expose `label_shortcuts(rows: &[(&str, &str)])` and let each app decide which rows it labels.

While there, `margin/src-tauri/Cargo.toml:57` asks for objc2-app-kit's `NSWritingToolsCoordinator`
feature and nothing in Margin uses it. Drop it.

**NSSpellChecker.** The closest pair in either Rust tree, 49 lines identical in order, 72% of the
smaller. `utf16_to_codepoint` is byte-identical, 12 lines: `margin/macspell.rs:14-25` against
`margin-editor/macspell.rs:54-65`. `check` is the same function either side of three differences:
Docs' named constants (`MAX_SUGGESTIONS`, `NO_DOCUMENT` at `:40-45`) against Margin's inline `5` and
`0`; Docs' `saturating_add` clamp at `:107` against Margin's overflow at `:53`; and Margin's
in-process custom word set at `:55-57`, a real product difference, since Docs learns into the system
(`learn` and `unlearn` at `:150-162`, absent from Margin). The crate takes an optional filter closure
and both fit. One `SpellIssue` (`margin-editor/src-tauri/src/dto.rs:179-185`) replaces Margin's local
`MacIssue` (`macspell.rs:7-12`, the same four fields).

**Notifications.** Margin Mail's `src-tauri/src/notify/macos.rs`, 286 lines, is the only working
system notification code in the suite and belongs here: the `UNUserNotificationCenter` delegate,
`permission` (`:107`), `request` (`:178`), `post` (`:205`), `open_settings` (`:257`), and the
settings read at `:120-162` that tells "denied" apart from "Show previews: Never". Its header at
`:3-14` says why it exists: `tauri-plugin-notification` posts through notify-rust and
mac-notification-sys, which use `NSUserNotificationCenter`, deprecated since 10.14. On macOS 26 that
API still answers, the delegate is told the notification was delivered and nothing appears; the app
never shows up under System Settings and the permission question is never asked. The module also
carries the two conditions of the working path: a bundled app with a real signature, and a request
before the first post.

Correction to repo-layout.md: the other three apps do not post through the broken plugin, they do not
post at all. Neither `tauri-plugin-notification` nor `@tauri-apps/plugin-notification` appears in
Margin's, Docs' or Calendar's manifests, and Calendar's `notify` at `App.tsx:27` is a toast helper
from `src/store/useToast`. What is true is that the plugin is what any of them would reach for the
first time they want a banner, and it silently does nothing. Mail keeps it for the non-macOS path
only (`notify.rs:484-486`), and the crate should keep that shape.

## Frontend

Worth sharing, in order of ratio:

- `src/editor/search.ts`, 206 of 217 code lines identical, 94%, the highest pair in either repo.
  Docs' header at `search.ts:6-7` says it was ported from Margin's and the only change of substance
  is the name of the last command; the rest of the diff is trailing commas and a return type.
  Outside the typesetting brief, and the most obviously extractable file in the suite.
- `ExportPreview.tsx`, 200 of 307, 65%. Share the frame-and-zoom shell, not the panel: the `Frame`
  interface, `measureEditorPane` against `measurePane` (Margin `:40-47`, Docs `:64-71`, identical but
  for the name), the zoom constants and step, the ctrl-wheel and gesture accumulator (Margin
  `:75-129`, Docs `:117-180`, one identifier apart), the pdf.js render loop with `RenderTask`
  cancellation, and the `ResizeObserver` on `.editor-pane`. Start from Docs': StrictMode-safe
  compile-once refs (`:113-135`), lazy pages via `AHEAD`, the overlay key context.
- `margin-editor/src/pdfjs.ts`, 27 lines, dynamic-imports pdf.js and its worker and caches the
  promise. Copy it into Margin whether or not anything else here is shared.
- `ProofPopover.tsx`, 29 of 68, 42%. Both portal to `document.body`, clamp to the window, humanise
  Harper's CamelCase kind the same way (Margin `:6-8` `humanize`, Docs `:57-63` `readableKind`) and
  render `""` as "Remove" (Margin `:58`, Docs `:70`). A `@margin/ui` primitive with the action row as
  a prop, since "Remember" and "Learn Spelling" write to different places.
- `src/escape.ts`, 36 lines byte-identical in three repos, and `src/editor/paste.ts` at 72%.

Not worth it:

- The markdown and remark stack, `margin-editor/src/markdown`, 2737 non-test lines plus roughly 8400
  of tests written against `src/model/schema.ts` as a frozen contract, and EPUB,
  `margin/src-tauri/src/epub.rs`, 88 lines of zip plumbing with one consumer. See the last table.
- `src/editor/extensions.ts`: twelve lines identical and they are the StarterKit import and the
  `configure` skeleton, since Docs switches StarterKit's schema off entirely at `:128-147`. Same for
  the TipTap pins (Margin `^3.27.1`, Docs `3.30.2`, Mail `^3.31.2`).
- `src/proofing.ts` and `src/editor/proofing.ts`, 13% and 0%. The same offset-mapping problem solved
  twice in incompatible shapes: Margin flattens the document to one string with a `Segment` table
  (`proofing.ts:39-73`), Docs maps per block inside 8000-character batches (`proofing.ts:154-251`).
  Docs' is better, but converging them is a rewrite rather than an extraction.

## Per app, in order

**Shared repo, first.** `crates/stubs/burn-cuda` and `crates/stubs/cubecl-cpu` with the versions and
feature lists as they stand. `margin-grammar` with the harper pin, `build_harper`, Docs'
`collect_grammar`, `GrammarIssue` and the lockfile test. `margin-mac` with the menu walk, the
availability probe, `check`/`learn`/`unlearn`, `SpellIssue` and Mail's notification module.
`margin-typeset` with Docs' `fonts.rs` (minus the dead `installed()` call at `:88` and its comment),
Docs' compile path, the diagnostic formatter, the warning aggregator, image handling, and the nine
static instances with their provenance. `@margin/fonts` as `shared/src/fonts.ts`, `shared/css` and
`shared/fonts` moved intact plus `sync-fonts --rust`; `@margin/typeset` for `str`, `sanitize`, the
extension sniffer and the shared escape vectors.

**Margin Docs, second, because it is the donor and its diff should be a deletion.** Delete
`fonts.rs`, `macspell.rs`, `writingtools.rs`, `grammar.rs` and `stubs/`; keep `spell.rs` as the
platform shim over `margin-mac`; reduce `pdf.rs` to the mitex two-pass and its own preamble, calling
`margin_typeset::compile`. Rewrite the patch block to the git form and add the lockfile test. If
Docs does not build cleanly on the crates, the crates are wrong, and finding that out here costs one
repo rather than two.

**Margin, third, and this is where the user-visible fixes land.** Take `margin-typeset` and the nine
static instances and headings stop exporting at 400; take the source-path dedup and a system family
stops being read four times; take `margin-grammar` and the inverted-span panic and the whitespace
underlines go; take `margin-mac` and the `NSNotFound` overflow goes, with the custom dictionary
staying as a filter passed in. Replace `str` at `export/typst.ts:36-38` with `@margin/typeset`'s and
backslash sequences stop appearing in exported code spans. Copy `pdfjs.ts`, add the fallback preamble
so code blocks and formulas have a face, drop the unused feature at `Cargo.toml:57`.

**Margin Mail, last and smallest.** Delete `system_fonts` from `settings.rs:179-189` and call
`margin_typeset::fonts::system_families()` with `default-features = false`. Move `notify/macos.rs`
into `margin-mac`; the 601-line test file stays in Mail, because what it tests is which arrivals are
worth announcing, not how a banner is posted. Nothing else in Mail touches this stack.

**Margin Calendar.** Nothing: no typst, no fontdb, no harper, no NSSpellChecker. It joins only if it
gains reminders, at which point it takes `margin-mac` for the notification path.

## What stays per app, and why

| Stays | Where | Measured reason |
| --- | --- | --- |
| The Typst converter and preamble | `src/export/typst.ts` in both | 9 of 335 lines shared, 2%. A book with trim sizes and a generated TOC against an A4 document with callouts and mitex. |
| Trim size, margins, running heads | each preamble | Product decisions. Margin has four print trim sizes at `typst.ts:16-21`; Docs has A4. |
| The mitex two-pass and its vendored wasm | Docs `pdf.rs:84-113, 344-379` | Margin has no math. Adding it to the crate would put a wasm binary in three apps that cannot use it. |
| The custom dictionary file | Margin `proofing.rs` | A deliberate product difference: Margin keeps its own, Docs learns into the system. The crate takes a filter, not a policy. |
| The Writing Tools key equivalents | each app's `writingtools.rs` caller | Docs' refusal is guarded by `src/editor/writing.ts`, which Margin does not have. Same code, different correct answer. |
| The markdown stack | Docs `src/markdown` | 2737 lines against a frozen schema contract. No other app has markdown. |
| EPUB | Margin `src-tauri/src/epub.rs` | 88 lines, one consumer, no second producer or reader in the suite. |
| The proofing driver and offset mapping | both `src/editor/proofing.ts` | 13% overlap. Two incompatible shapes; converging them is a rewrite, and it is not a precondition for any crate here. |
| Notification content and arrival rules | Mail `notify.rs` | 585 lines of "which of these is worth saying out loud", which is entirely Mail's product. Only the 286-line posting layer moves. |
| TipTap versions | all three | Docs is pinned exactly for the markdown contract; Mail's editor is 95 lines. |
