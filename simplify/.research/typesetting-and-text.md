# Typesetting and text: what margin and margin-docs actually share

Scope: the Typst PDF pipeline, fontdb loading, NSSpellChecker, Apple Writing Tools, harper-core
grammar, and the frontend that drives all of it. Four apps were checked. margin-calendar has none
of this: no typst, no fontdb, no harper, no NSSpellChecker in
`/Users/pj/Workspace/projects/python/margin-caledar/src-tauri/Cargo.toml`. margin-mail has exactly
one fingerprint, a fontdb family list. So this is a two-app problem between margin and margin-docs,
with one eight-line cameo from mail.

Method for the percentages below: comments and blank lines stripped, then a longest-common-
subsequence over the remaining lines. "Identical in order" means the same line, same order, both
files.

## The headline

Nothing in either Rust tree is a copy that could be lifted as-is. Every pair started as a copy and
then one side moved. margin-docs is ahead on all six Rust files and on four of the six frontend
ones, and in three places margin is not merely behind but carries a bug that margin-docs already
found, wrote a paragraph about, and fixed. The duplication that is worth extracting is small and
boring; the duplication that is expensive is the divergence, and a shared crate is the only thing
that would have stopped it.

Both Cargo.lock files pin the same graph: harper-core 2.5.0, typst 0.14.2, fontdb 0.23.0,
burn-cuda 0.19.1, cubecl-cpu 0.8.1, 964 packages in margin and 962 in margin-docs. Two copies of a
960-package dependency graph resolved to the same versions by hand.

## PDF export

`margin/src-tauri/src/pdf.rs` is 129 lines, 116 of code.
`margin-editor/src-tauri/src/pdf.rs` is 455 lines, 275 of code.
Identical in order: 37 lines, 31% of the smaller file.

Truly byte-identical, verified with diff:

- Diagnostic formatting. `margin/src-tauri/src/pdf.rs:114-129` and
  `margin-editor/src-tauri/src/pdf.rs:440-455` are the same 16 lines, character for character. Only
  the function name differs (`format_source_diagnostics` against `format_diagnostics`). Severity to
  string, message, hints indented two spaces, joined by newline.
- The compile call shape. `Warned { output, warnings } = engine.compile()` then
  `typst_pdf::pdf(&document, &Default::default())`: margin pdf.rs:78-81, docs pdf.rs:272-281.
- The bundled-family dispatch. Same four ids (`eb-garamond`, `lora`, `source-serif`, `fraunces`)
  mapping to the same eight `include_bytes!` of `../../public/fonts/*-VF.ttf`: margin pdf.rs:22-35,
  docs pdf.rs:52-82. Same list, different shape (`&[&[u8]]` statics in both, arranged differently).

Everything else has diverged, and the direction is one-way.

**Font strategy is where the drift costs the user a PDF.** margin loads the variable files for
Literata and Hanken and hands them to Typst (pdf.rs:9-20, 57-62). margin-docs cut nine static
instances into `margin-editor/src-tauri/fonts/` and loads those (pdf.rs:32-42), with the reason
written at pdf.rs:24-31: Typst does not support a variable axis, warns that it does not, and lays
out at the default instance regardless of the weight asked for. So in margin every heading, every
bold run and every callout label exports at weight 400. margin-docs' PDFs have the hierarchy the
author sees on screen; margin's do not. margin-docs even documents the residual caveat for the
other four families (pdf.rs:44-51), which margin has never noticed. This is the single largest
quality gap found anywhere in this audit, it is 1.6M of static cuts plus a PROVENANCE.md, and it
exists in one repo only.

Present in margin-docs, absent from margin:

- A vendored mitex 0.2.5 served on `/mitex/` as static source files, with a wasm binary
  (pdf.rs:84-98), a stand-in `lib.typ` for a second attempt (pdf.rs:105-113), and the two-pass
  compile that uses it (pdf.rs:344-379). A formula mitex cannot parse degrades to literal source
  instead of killing the export.
- Placeholder images, one per format Typst picks from an extension (pdf.rs:115-144), so an
  unreadable image is a gap rather than a failed export.
- A root-guarded filesystem read for images with no inline bytes (pdf.rs:146-163, calling
  `fs::resolve_in_roots`). margin's `ImageInput.data` is a mandatory base64 string (pdf.rs:37-41)
  and there is no file path in the protocol at all, so margin has no exposure here but also cannot
  export an image it does not already hold in memory.
- The font preamble, written in Rust because only Rust knows which families were found
  (pdf.rs:165-194). Note pdf.rs:176-181: Typst sets every equation in "New Computer Modern Math"
  with fallback off, so on a machine without it one formula is a hard compile error. margin has no
  math and no monospace preamble at all.
- Warning aggregation with counts and a `kind` (pdf.rs:196-244 plus `PdfWarning` in dto.rs:217-222).
  margin emits one joined string on the same `pdf-warnings` event (pdf.rs:100-102).
- `pdf_write` with the guard that refuses to overwrite a markdown or text file (pdf.rs:411-437).

The escaping boundary is on the frontend in both, covered below.

## Font loading

`margin/src-tauri/src/fonts.rs` is 47 lines, 43 of code.
`margin-editor/src-tauri/src/fonts.rs` is 182 lines, 113 of code.
Identical in order: 17 lines, 39% of the smaller.

The three-way duplication is here and it is trivially extractable. Listing every installed family
is the same eight lines in three crates:

- `margin/src-tauri/src/fonts.rs:11-18` (`list_system_fonts`)
- `margin-editor/src-tauri/src/fonts.rs:160-167` (`fonts_list_system`), byte-identical to margin's
- `margin-mail/src-tauri/src/settings.rs:180-188` (`system_fonts`), the same algorithm with
  `system_db()` inlined and `names` renamed to `families`

All three do `db.faces()`, `families.first()`, clone the name, sort, dedup. All three are
`#[tauri::command(async)]`. mail is on fontdb 0.24 (`margin-mail/src-tauri/Cargo.toml:70`), the
other two on 0.23. Nothing in this function changed between those versions.

The per-family loader is the second shared piece: the same four `(Weight, Style)` pairs, the same
`Query { families: &[Family::Name(family)], weight, style, ..Default }`, the same
`with_face_data(id, |data, _| data.to_vec())`. margin fonts.rs:21-46, docs fonts.rs:69-95.

Two drifts, both in margin-docs' favour:

- Deduplication key. margin dedups by `fontdb::ID` (fonts.rs:30, 39), which is per face. docs dedups
  by source file path (fonts.rs:55-61, 88). `with_face_data` hands back the whole file, so a `.ttc`
  holding regular, italic, bold and bold-italic is read and shipped to Typst four times by margin
  and once by docs. On a system family that is a collection, margin sends four copies of the same
  multi-megabyte blob across the compile.
- The whole `Fallbacks` machinery (fonts.rs:19-42 for the family lists, 97-143 for the collection)
  has no counterpart in margin. margin names no monospace and no math family, so a code block or a
  formula in a margin export gets whatever Typst defaults to.

One correction worth carrying into any shared crate: the comment at
`margin-editor/src-tauri/src/fonts.rs:84-86` says fontdb "answers a query with its closest match
rather than with nothing, so a family that is not installed comes back as some unrelated face".
That is not what fontdb 0.23 does. `Database::query`
(`~/.cargo/registry/src/index.crates.io-*/fontdb-0.23.0/src/lib.rs:661-679`) filters candidates by
exact family-name equality and returns `None` when the list is empty. The `installed()` guard at
fonts.rs:88 therefore cannot fire, and since it compares case-insensitively while the query compares
exactly, it is strictly weaker than the filter that already ran. It is live and useful at
fonts.rs:139, where it is asked about `db.faces()` directly. Harmless dead code, but the comment
would mislead whoever writes the shared version.

## Spellcheck

`margin/src-tauri/src/macspell.rs` is 73 lines, 68 of code.
`margin-editor/src-tauri/src/macspell.rs` is 162 lines, 82 of code.
Identical in order: 49 lines, 72% of the smaller. This is the closest pair in either Rust tree.

`utf16_to_codepoint` is byte-identical, 12 lines: margin macspell.rs:14-25, docs macspell.rs:54-65.

`check` is the same function with three changes. Both build the NSString, ask
`checkString_range_types_options_inSpellDocumentWithTag_orthography_wordCount` for
`Spelling | Link`, skip non-spelling results, map both ends of the NSRange through the table, cut
the word out of the caller's own `chars`, and take at most five guesses.

- margin filters against an in-process custom word set (macspell.rs:55-57) because it keeps its own
  dictionary file. docs has no custom set: learning writes to the system
  (`macspell.rs:143-162`, `learn` and `unlearn`, absent from margin entirely).
- docs names its constants (`MAX_SUGGESTIONS`, `NO_DOCUMENT` at macspell.rs:40-45); margin inlines
  `5` and `0`.
- The end-offset clamp. margin: `map[(range.location + range.length).min(len)]` (macspell.rs:53).
  docs: `map[range.location.saturating_add(range.length).min(len)]` (macspell.rs:107). The add
  happens before the clamp in margin, so an NSRange carrying `NSNotFound` as its location panics in
  debug and wraps in release. Small, real, and already fixed once.

margin returns a local `MacIssue` (macspell.rs:7-12); docs returns the shared `SpellIssue` from
dto.rs:179-185. Same four fields.

docs also has `spell.rs` (95 lines, 28 of code), a platform shim giving the frontend four commands
on every target with a `no_checker` module for non-macOS. margin has no equivalent; its
non-macOS path is `#[cfg]` branches inside `proofing.rs` (proofing.rs:91-96, 105-166, 232-238) plus
a bundled 550K SCOWL Hunspell dictionary under `margin/src-tauri/resources/dictionaries/en/`.

## Apple Writing Tools

`margin/src-tauri/src/writingtools.rs` is 81 lines, 68 of code.
`margin-editor/src-tauri/src/writingtools.rs` is 146 lines, 83 of code.
Identical in order: 42 lines, 61% of the smaller.

Byte-identical, verified: `submenu_named` and `edit_menu` together, 21 lines. margin
writingtools.rs:10-30 against docs writingtools.rs:23-43. `writing_tools_menu` is the same
one-liner in both (margin:32-34, docs:45-47). That is the entire AppKit menu-walking layer, and it
is the cleanest candidate for extraction in the whole audit: it takes a `MainThreadMarker` and a
title and it knows nothing about either app.

The divergence is a deliberate disagreement, and it is documented. margin puts Shift+Option+F and
Shift+Option+R on Apple's own Proofread and Rewrite rows (writingtools.rs:8, 36-49). margin-docs
refuses to, and writingtools.rs:85-91 says why: AppKit performs a key equivalent by firing the menu
item directly, so a chord on the system's row reaches Writing Tools without passing the selection
guard in `src/editor/writing.ts`, and that guard is refusing selections that corrupt the file. docs
puts the chords on its own Edit rows instead. margin has no such guard, so it is not currently
wrong for margin, but the reasoning is the sibling's and margin has never seen it.

docs also adds an availability probe: a `SUBMENU_SEEN` atomic set at setup (writingtools.rs:13-14,
92-95) and a `writing_available` command (106-120), so the frontend can say the machine has no
Apple Intelligence instead of offering a button that does nothing. margin's `perform` returns `()`
and silently no-ops when the row is missing (writingtools.rs:51-61); docs' returns
`Result<(), String>` with three distinct messages (58-73).

One manifest oddity: `margin/src-tauri/Cargo.toml:57` asks for the `NSWritingToolsCoordinator`
feature of objc2-app-kit and nothing in margin uses it. margin-docs names exactly that in a comment
at `margin-editor/src-tauri/Cargo.toml:67`.

## Grammar

margin has one module doing spelling, grammar and the custom dictionary:
`margin/src-tauri/src/proofing.rs`, 275 lines, 239 of code. margin-docs splits the same work three
ways: `grammar.rs` (128 lines, 58 of code), `spell.rs` (95 lines, 28), `macspell.rs`.

Comparing the harper parts only, `proofing.rs` against `grammar.rs`: 33 lines identical in order,
56% of the smaller.

`build_harper` is the same five lines of code in both, margin proofing.rs:98-103 and docs
grammar.rs:41-47: `FstDictionary::curated()`, `LintGroup::new_curated(dict.clone(),
Dialect::American)`, `set_rule_enabled("SpellCheck", false)`. Same reasoning in both, that the
system checker does spelling better.

`collect_grammar` is the same walk: `Document::new_plain_english(text, dict)`, iterate lints, slice
`existing` out of `chars`, map `Suggestion::ReplaceWith` / `InsertAfter` / `Remove` the same three
ways, truncate to five. margin proofing.rs:168-194, docs grammar.rs:74-127.

Two differences, both docs ahead:

- Span clamping. margin clamps start and end against `chars.len()` independently
  (proofing.rs:171-172). docs clamps end first, then start against end (grammar.rs:84-85), with the
  comment saying why: a foreign engine walking user prose that hands back an inverted span makes
  `chars[start..end]` a panic in margin and a no-op in docs.
- docs drops lints whose span is nothing but whitespace (grammar.rs:96-98), naming Harper's "French
  spaces" rule specifically. margin draws them, so a margin user gets an invisible underline over
  two spaces that cannot be clicked.

Engine lifecycle differs without either being wrong: margin lazily fills a Tauri-managed
`Mutex<Option<Engine>>` (proofing.rs:36-40, 203-216); docs uses a module-level
`LazyLock<Mutex<Harper>>` (grammar.rs:39).

## The [patch.crates-io] trap

Both crates carry the same two-line patch, and the four stub files behind it are code-identical.

- `margin/src-tauri/Cargo.toml:70-72` and `margin-editor/src-tauri/Cargo.toml:98-100`: the same
  `burn-cuda = { path = "stubs/burn-cuda" }` and `cubecl-cpu = { path = "stubs/cubecl-cpu" }`.
- `harper-core = { version = "=2.5.0", features = ["concurrent"] }`, margin Cargo.toml:38 and docs
  Cargo.toml:57. Both carry a comment above it saying the pin is exact because the stubs are tied to
  this version's burn/cubecl graph and a minor bump could silently invalidate the patch.
- `[profile.dev.package."*"] opt-level = 3`, margin Cargo.toml:78-79 and docs Cargo.toml:107-108,
  with the same justification (harper's burn-ndarray POS tagger is roughly ten times slower
  unoptimized).
- The stubs themselves: `stubs/burn-cuda/Cargo.toml` (21 lines) and `stubs/cubecl-cpu/Cargo.toml`
  (16 lines). Diffing them with comment lines removed produces no output. The feature lists that
  have to match burn's `burn-cuda?/...` references (`std`, `doc`, `fusion`, `autotune`,
  `autotune-checks`) and the pinned stub versions (burn-cuda 0.19.1, cubecl-cpu 0.8.1) are the same
  in both.

The only differences across all four stub files are prose: margin says "broken/heavy" where docs
says "heavy", margin's cubecl-cpu comment blames failing tracel-llvm prereleases where docs blames
build cost, and margin uses an em dash in the two `lib.rs` one-liners where docs uses a colon.

Why this is a trap rather than merely duplication. Cargo treats an unused `[patch]` as a warning,
not an error. The patch's validity depends on harper-core 2.5.0's exact transitive graph. Whoever
bumps harper-core in one repo has to know to re-audit the stub versions, and there is no test in
either repo asserting the patch is still applied. A stale stub does not fail the build; it quietly
pulls several hundred crates and an LLVM toolchain back into the graph, and the symptom is a build
that got slow. Two copies means the bump happens twice, on two different days, by which time the
comment explaining the pin has been read once and skipped once.

This is the strongest single argument for a shared crate in the audit: not because the code is
large, but because the fragile fact needs one owner. A `margin-grammar` crate would carry the
harper pin, the two stubs, the patch section, and one integration test asserting `cubecl-cpu`
resolves to the stub version.

## Frontend

### Typst converters: no shared code, and one real bug

`margin/src/export/typst.ts` is 421 lines (335 of code).
`margin-editor/src/export/typst.ts` is 808 lines (486 of code).
Identical in order: 9 lines, 2%.

They are genuinely different documents. margin writes a book: trim sizes (typst.ts:16-21), a title
page, chapter and part openers, a generated table of contents that queries `<chap>` metadata
(typst.ts:130-239). margin-docs writes an A4 document: callouts, toggles, task lists, tables, code
surfaces, mermaid diagrams, mitex math (typst.ts:206-327). Nothing about either preamble is
shareable and neither should be.

The one identical fragment is the image extension sniffer, 5 lines: margin typst.ts:240-244
(`imageExtension`) and docs typst.ts:342-347 (`extensionOf`).

The escaping philosophies are opposites, and docs' header says so directly (typst.ts:6-14): it has
no `esc` "that sprinkles backslashes through markup the way the sibling book exporter does".
Nothing in docs writes user text into markup; every character leaves through `str`
(typst.ts:98-110), which builds a Typst string literal, and a string literal has no syntax inside
it. margin escapes into markup with a character class (typst.ts:23-34): `INLINE_SPECIAL` covers
`\ # $ * _ \` < > @ ~ ( ) [ ]`, `guardLineStart` handles a leading `= - + /` and a leading `1.`.

margin's `str` is `JSON.stringify` (typst.ts:36-38), and that is a bug. Typst's string literal
resolves exactly `\\`, `\"`, `\n`, `\r`, `\t` and `\u{...}`, and an escape it does not recognise it
copies through as literal text
(`typst-syntax-0.14.2/src/ast.rs:1248-1268`, the `_ => out.push_str(s.from(start))` arm).
JSON.stringify emits `\b`, `\f`, and `\uXXXX` for every other C0 control, for U+2028 and
U+2029, and for a lone surrogate. Every one of those lands in the PDF as the literal backslash
sequence instead of the character it stood for. There is no compile error to catch it. docs' `str`
(typst.ts:98-110) emits `\u{...}` and runs `sanitize` (typst.ts:80-87) to replace lone surrogates
first, precisely because they survive neither the IPC JSON nor UTF-8 on the other side. margin
carries a book title straight into `str` at typst.ts:136, and an href at typst.ts:57.

Secondary: margin does not neutralise Typst's markup shorthands, so `--` in a manuscript becomes an
en dash and `...` becomes an ellipsis in the PDF without the author asking. Arguably desirable in a
book; it is silent either way.

Image handling also diverged. margin only ever sends inline base64 from data URLs
(typst.ts:275-301, `ImageInput.data: string` at `margin/src/ipc.ts:10-13`). docs sends a path for a
file-backed image and bytes only for things with no file, such as a rendered mermaid SVG
(typst.ts:357-396, `ImageInput.data: string | null` at `margin-editor/src/ipc.ts:244-248`). The
path variant is what makes the root guard in docs' pdf.rs necessary and is why the two DTOs cannot
merge without a decision.

### ExportPreview: the most duplicated file in either repo

`margin/src/components/ExportPreview.tsx` is 336 lines (307 of code).
`margin-editor/src/components/ExportPreview.tsx` is 448 lines (342 of code).
Identical in order: 200 lines, 65% of the smaller.

docs' header (lines 1-19) states outright that the sibling answers Export with the same panel for
the same reason. Shared verbatim or near-verbatim: the `Frame` interface, `measureEditorPane`
against `measurePane` (margin:40-47, docs:64-71, identical but for the name), the zoom constants
and step, the whole ctrl-wheel and gesture accumulator block (margin:75-129, docs:117-180, differing
in one identifier), the pdf.js page render loop with `RenderTask` cancellation, the `ResizeObserver`
on `.editor-pane`.

The drift is mostly docs adding: StrictMode-safe compile-once refs (docs:113-135), lazy pages via
`AHEAD`, an overlay key context, toast integration. margin adds `unsupportedScripts` messaging
(margin:129 onward, typst.ts:307-334) which docs does not have.

pdf.js loading is the one place the drift is a cost margin pays every launch. margin imports
`pdfjs-dist` and the worker URL at module scope in ExportPreview.tsx and sets
`GlobalWorkerOptions.workerSrc` at line 18, so roughly 1.5M of reader is in the main bundle whether
or not anyone opens the panel. docs has `margin-editor/src/pdfjs.ts`, 27 lines, which dynamic-
imports both and caches the promise (pdfjs.ts:17-26). Two surfaces use it there. docs is ahead and
the fix is 27 lines.

### Proofing on the frontend: same problem, two solutions, no shared code

- `margin/src/editor/proofing.ts` (139 lines) is a decoration plugin only. The driving loop lives in
  `margin/src/components/EditorView.tsx:137-255`: a debounce, a whole-document pass, a `setTimeout`.
- `margin-editor/src/editor/proofing.ts` (642 lines, 344 of code) holds the plugin plus block
  batching into 8000-character runs (proofing.ts:85-88, 201-225), two module-level caches keyed by
  paragraph text (125-126, 253-259), a pass sequence number, and keyboard menu opening. 17 lines
  identical in order, 13%. The only common line of substance is `Decoration.inline` over a mapped
  span.
- `margin/src/proofing.ts` (112 lines) has no counterpart at all. `docText` and `mapOffset`
  (proofing.ts:39-73) flatten the whole document to one string with a `Segment` table to map code
  point offsets back to ProseMirror positions. docs solves the same offset problem per block inside
  proofing.ts:154-251. Genuinely the same problem solved twice, in incompatible shapes.

`ProofPopover.tsx`: margin 76 lines (68 of code), docs 292 (184). 29 identical in order, 42%. Both
portal to `document.body`, clamp to the window, humanise Harper's CamelCase category the same way
(margin:6-8 `humanize`, docs:57-63 `readableKind`, docs adding `0-9` to the class), render `""` as
"Remove" (margin:58, docs:70), and lay out suggestions then actions. docs adds arrow-key walking,
the pointer-versus-chord focus split, and a store. margin's "Remember" writes to its own dictionary
file; docs' "Learn Spelling" writes to the system.

### Editor and TipTap

`src/editor/extensions.ts`: margin 28 lines, docs 180. 12 lines identical in order, and those 12
are the StarterKit and Placeholder imports and the `configure` skeleton. The schemas are not
comparable: margin uses StarterKit's nodes plus Figure, ParagraphIndent, TextAlign; docs generates
one TipTap extension per entry in its own frozen `src/model/schema.ts` and switches all of
StarterKit's schema off (extensions.ts:128-147), because the markdown bridge is written against
that contract.

margin-mail's editor is `margin-mail/src/screens/Editor.tsx`, 95 lines: one `StarterKit.configure`
with `heading: false` and `horizontalRule: false` (Editor.tsx:41-48), no toolbar, no proofing, no
export. Its own comment (Editor.tsx:8-11) points at margin's extensions.ts as the model. Nothing to
share but the version pin, and the pins already disagree: margin `^3.27.1`, margin-docs pinned
`3.30.2`, margin-mail `^3.31.2`, across all five `@tiptap/*` packages.

Two frontend pairs outside the brief are worth naming because they are more duplicated than
anything in it:

- `src/editor/search.ts`: 217 against 235 code lines, 206 identical in order, 94%. docs' header
  (search.ts:6-7) says "Ported from margin's editor/search.ts. The only change of substance is the
  name of the last command". The rest of the diff is prettier's trailing commas and a return type.
  This is the highest-similarity file pair in either repo.
- `src/editor/paste.ts`: 70 against 163 code lines, 51 identical, 72%.
- `src/escape.ts` is byte-identical, 36 lines, in margin, margin-docs and margin-calendar.

### Fonts on the frontend are already shared, and only there

`margin/shared/src/fonts.ts` (233 lines) is the single list of the six bundled families, the
`FontRef` encoding, the pairings and `fontsUsed`. All three UI apps import it: margin, margin-docs
and margin-mail (`margin-mail/src/screens/Settings.tsx:7-12`). This is the model for what a shared
Rust crate should look like, and its own header (fonts.ts:3-7) makes the argument: four lists in two
repos cannot be kept honest by hand.

The Rust side is the half that never got done. The same six ids are re-declared as `include_bytes!`
in margin pdf.rs:9-25 and docs pdf.rs:52-82, and the family list command exists three times. The
bytes are duplicated on disk five times: 6.8M in each of `margin/public/fonts`,
`margin-editor/public/fonts`, `margin-mail/public/fonts` and the `margin/shared/fonts` master, plus
1.6M of static cuts in `margin-editor/src-tauri/fonts`. `margin/shared/bin/sync-fonts.mjs` is the
copier.

## EPUB

`margin/src-tauri/src/epub.rs`, 88 lines. Two commands: `package_epub` zips a list of
path/data/encoding records, `unzip_epub` reads one back. The only EPUB-specific facts are the
stored-not-deflated `mimetype` entry written first (epub.rs:29-31) and the text-extension list
(epub.rs:15-23). Nothing in it knows what a Book is.

So it is general in principle and margin-only in practice. It has one consumer, the frontend that
feeds it is entirely Book-shaped (`margin/src/export/epub.ts` at 18733 bytes and
`margin/src/import/epub.ts` at 17738), and no other app in the suite produces or reads an EPUB.
Sixty lines of zip plumbing is not a crate. Leave it.

## Markdown and the remark stack

`margin-editor/src/markdown` is 2737 non-test lines (index 157, parse 597, serialize 1471, handlers
420, frontmatter 92) plus roughly 8400 lines of tests. It depends on unified, remark-parse,
remark-stringify, remark-gfm, remark-frontmatter, remark-math and mdast-util-to-markdown, none of
which appear in margin or margin-mail.

It is not shareable and should not be made so. Every line of it is written against
`margin-editor/src/model/schema.ts`, which the module's header (index.ts:29-36) treats as a frozen
contract, and its four stated invariants (opening writes nothing, serializing is stable, frontmatter
passes through opaque, nothing is silently dropped) are promises about a folder of markdown files.
margin has no markdown at any point in its pipeline and mail composes HTML. If mail ever gained
markdown compose it would want a serializer, not this parser, and the schema it serialized from
would be StarterKit's rather than the contract's.

## What is worth extracting

Ordered by ratio of risk removed to work done.

1. The harper pin, the two stub crates and the `[patch.crates-io]` block. Smallest code, largest
   consequence, and the only duplication here that fails silently and expensively.
2. The AppKit menu walk from writingtools.rs: `submenu_named`, `edit_menu`, `writing_tools_menu`,
   21 lines already byte-identical, plus the availability probe docs has and margin does not.
3. fontdb: `system_db`, the installed-family list (three copies today), the four-style face loader,
   and the source-path dedup key margin is missing. Take docs' version, drop the incorrect comment
   at fonts.rs:84-86.
4. `utf16_to_codepoint` and the NSSpellChecker `check` body, 12 lines already identical and about 40
   more that differ only in constants and the return type. Needs one shared `SpellIssue`.
5. Typst diagnostic formatting, 16 identical lines, and the compile-and-render call. Not the
   preambles and not the converters.
6. The nine static font cuts, so margin's PDFs stop exporting every heading at weight 400. This is
   an asset and a build step more than a crate, but it is the change a reader of the two PDFs would
   notice first.

Frontend, separately from any Rust crate: pdf.js lazy loading (27 lines, docs already has it),
ExportPreview's frame-and-zoom shell (65% identical), and margin's `str` bug in export/typst.ts:36.
