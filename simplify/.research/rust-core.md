# Rust core plumbing (research, 2026-09-06)

What the four apps have in common below the features: `library.rs`, `lib.rs`, `dto.rs`, SQLite,
logging, settings, filesystem helpers, error shapes, updates, async, and dependency versions.
Everything here was read off disk. Google/OAuth, typesetting and spellcheck are other notes.

Sizes for reference: 2,190 Rust lines in Margin, 8,490 in Calendar, 5,365 in Docs, 45,884 in Mail.
Mail's number includes 8,045 lines of `tests.rs` files; Calendar's includes 802.

## The verdict first

| Area | Apps that have it | How close, really | One crate? |
|---|---|---|---|
| `lib.rs` builder and menu | 4 | ~120 lines per app verbatim identical | **Yes**, the biggest single win |
| Logging | 1 (Mail) | three apps have nothing | **Yes**, and it fixes a real gap |
| Updates | 1.5 (Margin, plus `packaged_by` in two) | Margin's `updates.rs` is already app-agnostic | **Yes**, cheap |
| SQLite | 3 | ~100 duplicated lines out of ~8,500 | **Yes, small**, for the two bugs it fixes |
| `library.rs` | 4 | 5 identical lines, then four different files | No |
| `dto.rs` conventions | 3 | one convention, rigidly held, nothing to extract | No |
| Settings | 1 (Mail) | the other three keep prefs in `localStorage` | No |
| Filesystem helpers | 2.5 | three genuinely different algorithms | No |
| Error types | 4 | already uniform, nothing to fix | No |
| Async | 3 | one shared idea (`Sink`), 12 lines | No |

Three defects found on the way, listed at the end of the `lib.rs` section.

## library.rs

Line counts: Margin 146, Calendar 9, Docs 9, Mail 54.

Calendar's and Docs' are **byte-identical files** (`diff` returns nothing): nine lines containing
only `app_data_dir`. Mail's is that same function plus `atomic_write` and a test. Margin's is a
different file that happens to share the name: `BookSummary` (margin `library.rs:8-14`), the four
book commands, and `app_data_dir` at `library.rs:49-53`.

The genuinely shared part is five lines, identical in all four
(margin `library.rs:49-53`, calendar `library.rs:5-9`, docs `library.rs:5-9`, mail `library.rs:8-12`):

```rust
pub fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
```

It should move into whatever shared crate exists for other reasons. It is not a reason to create
one. Everything else in Margin's `library.rs` is the book library and belongs to Margin.

## lib.rs: the Tauri builder

Line counts: Margin 256, Calendar 354, Docs 365, Mail 475. 1,450 lines total.

Measured overlap: 73 distinct non-comment lines appear **verbatim in all four files**. Counting
occurrences, that is 114 lines of Margin's `lib.rs`, 123 of Calendar's, 120 of Docs' and 124 of
Mail's, which is 48%, 44%, 41% and 32% of each file's code lines. Excluding trivial brace lines it
is still 73 to 81 lines each. Roughly 480 lines of duplicated boilerplate across the suite.

The identical blocks, in order:

**`main.rs`.** Six lines, identical in all four but for the crate name. Nothing to do here; Tauri
requires it.

**The builder prologue.** Margin `lib.rs:149-162`, Calendar `250-263`, Docs `250-266`, Mail
`253-268`. `generate_context!` first, `#[cfg_attr(mobile, allow(unused_mut))]`, then:

```rust
#[cfg(desktop)]
{
    builder = builder.plugin(tauri_plugin_process::init());
    if context.config().plugins.0.contains_key("updater") {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }
}
```

Verbatim four times. Two copies say so in a comment: Calendar `lib.rs:248-249` and Mail
`lib.rs:251-252` both read "Ported from margin's lib.rs".

**The menu scaffold.** `Menu::default(handle)`, the `submenus` collect, the `find_submenu` closure,
the `match find_submenu("File")` with its `prepend_items` / `SubmenuBuilder` arms, the Edit and Help
appends, the macOS app-submenu insert block and the non-macOS fallback. Margin `lib.rs:46-60` and
`62-93`, Calendar `50-64` and `66-95`, Docs `100-114` and `115-146`, Mail `84-98` and `100-135`.
Pairwise diffs of the whole `build_menu`: Calendar against Mail is 49 differing lines out of 118 and
123. Margin against Calendar is 89. Docs is the outlier at 149 to 159, because it rebuilds the
macOS app submenu from scratch rather than patching Tauri's default (the reasoning is at Docs
`lib.rs:164-176` and is good).

**The menu event.** Identical line in all four: `app.emit("menu-action", event.id().0.as_str()).ok();`
(Margin `lib.rs:192`, Calendar `308`, Docs `318`, Mail `341`), inside an identical `matches!` guard
over a list of ids.

**`show_main_window`.** Calendar `lib.rs:227-234` and Mail `lib.rs:184-191` have identical bodies.

**`packaged_by`.** Calendar `lib.rs:239-244` and Mail `lib.rs:196-201`, identical including the doc
comment, differing only in the env var name (`MARGIN_CALENDAR_PACKAGED_BY` vs `MARGIN_MAIL_PACKAGED_BY`).

**`build.rs`.** Margin's and Docs' are the three-line default. Calendar's and Mail's are
byte-identical 29-line files with the same `embed_credentials` and the same comment.

### Where they genuinely must differ

The menu contents (ids, labels, accelerators, which submenus get extra rows), the `invoke_handler`
list, the `manage` calls and the body of `setup`, deep link registration (Calendar and Mail only),
and the iOS viewport fix and Android consent-tab watcher (Calendar only, see below).

### Three defects found while reading

**Margin Mail cannot compile for mobile.** `#[cfg_attr(mobile, tauri::mobile_entry_point)]` sits at
`lib.rs:203`, directly above `attach_account`, not above `pub fn run()` at `lib.rs:250`. Separately,
`setup` calls `listen_for_redirects` (`lib.rs:307`), `stop_uikit_shrinking_the_viewport` (`lib.rs:310`)
and `watch_for_the_consent_tab_closing` (`lib.rs:314`) under `cfg(mobile)`, `cfg(target_os = "ios")`
and `cfg(target_os = "android")`. None of the three is defined anywhere in the crate; grep returns
only the call sites. All three exist in Calendar (`lib.rs:149-172`, `185-199`, `213-220`) and were
evidently meant to be ported with the rest. The iOS and Android dependency blocks are in
`Cargo.toml` waiting for them.

**Four apps, three close behaviours.** Calendar and Mail prevent the close and hide the window
(Calendar `lib.rs:313-321`, Mail `346-354`), then restore it on `RunEvent::Reopen`. Margin lets the
window be destroyed but calls `api.prevent_exit()` and rebuilds the window from config on Reopen
(`lib.rs:231-238`, `241-256`). Docs calls `.run(context)` directly at `lib.rs:363`, so it has no
`RunEvent` closure and no `CloseRequested` handler anywhere in the crate: closing the window quits
the app. For a suite that shares a design language this is the kind of thing that should have one
answer, and a shared shell crate would force one.

**Capability drift.** Margin puts `core:window:allow-destroy` and `allow-start-dragging` in
`default.json`, which applies on every platform; the other three put them in `desktop.json`. Docs
additionally carries `core:window:allow-toggle-maximize`. Nothing is broken, but four hand-edited
copies of the same two files will keep drifting.

### What the crate would be

A `margin-shell` crate holding: the plugin prologue as `fn desktop_plugins(builder, context)`, the
menu scaffold as `fn standard_menu(handle, spec: &MenuSpec) -> tauri::Result<Menu<R>>` where
`MenuSpec` names the File rows, the extra Edit and View rows and the Help rows, the `menu-action`
forwarding, `show_main_window`, `hide_on_close`, `packaged_by(env_var)` and `app_data_dir`. Around
250 lines, deleting roughly 400 across the four apps, and it makes the close behaviour and the
capability set one decision instead of four.

## dto.rs and the IPC boundary

| | Lines | Structs | Enums | `rename_all = "camelCase"` |
|---|---|---|---|---|
| Margin | no `dto.rs` | types inline in their modules | 0 | 8 across the crate |
| Calendar | 181 | 10 | 1 | 11 |
| Docs | 243 | 15 | 0 | 15 |
| Mail | 927 | 40 | 11 | 43 |

The three `dto.rs` files open with the same two-line header ("The IPC contract. Every type here has
a matching declaration in src/ipc.ts. Both sides are frozen once written: implementation modules add
bodies, not fields."). Margin has no `dto.rs`, but follows the same convention where it matters:
`BookSummary` at `library.rs:8-14` is `#[derive(serde::Serialize)]` with `rename_all = "camelCase"`.

The convention is one convention and it is held rigidly: `#[derive(Debug, Clone, Serialize,
Deserialize)]` plus `#[serde(rename_all = "camelCase")]` on every type; `#[serde(default)]` on
patch and optional fields (9 in Margin, 55 in Calendar, 7 in Docs, 148 in Mail); `Option<T>` for
absent rather than a sentinel; `i64` epoch milliseconds for time; a string field with the legal
values in a doc comment (`/// idle | syncing | error`) instead of an enum. `#[serde(rename = ...)]`
appears exactly once in the whole suite (Calendar `dto.rs:36`, for `self`), and `skip_serializing_if`
twice, both in Mail. Mail is the only app with real enums, all `rename_all = "kebab-case"`.

**Is there a macro or crate here? No.** `#[serde(rename_all = "camelCase")]` is already the shortest
spelling of the thing; a derive macro would save one line per struct and put a proc-macro crate in
four build graphs. The duplication that costs something is on the other side: 65 structs across the
three `dto.rs` files each have a hand-written TypeScript interface in `src/ipc.ts` with nothing
checking that they agree, and the first 41 lines of Calendar's and Docs' `ipc.ts` are byte
identical. That is a codegen question (`ts-rs`, `tauri-specta`) for the frontend note.

The one type that genuinely repeats is the progress status: Calendar `SyncStatus` (`dto.rs:147-167`),
Docs `IndexStatus` (`dto.rs:87-109`), Mail `SyncStatus` (`dto.rs:861-893`). All three are
`phase: String` with the states in a doc comment, `error: Option<String>`, `message: Option<String>`,
progress counters, and a hand-written `Default` or `idle()` constructor. The common core is six
lines. Similarly `AuthEvent`: Calendar `dto.rs:172-181` and Mail `dto.rs:898-910`, where Mail's is
Calendar's plus `granted_scopes` and `missing_required`. Worth putting in a shared crate that exists
anyway. Not worth one on its own.

## SQLite

Present in three. Calendar `store/` is 1,351 lines (about 960 non-test). Docs `index.rs` is 1,771
lines of which only about 420 touch SQLite at all, the rest being fzy scoring, snippet windowing and
markdown parsing. Mail is 6,222 non-test lines across `db.rs`, `mirror/` and `state/`, plus two
`.sql` schema files. Call it 8,500 lines of database code.

**Genuinely duplicated: 80 to 120 lines.** These are not three copies of one layer, they are three
different databases in one house style. The specific overlaps:

- `app_data_dir`, as above.
- `version()`, byte-identical between Calendar `store/schema.rs:130-136` and Mail
  `mirror/schema.rs:127-135`, and again modulo the `state.` prefix at Mail `state/schema.rs:56-64`.
- The `meta` upsert. Calendar `store/write.rs:214-228`, Mail `mirror/write.rs:57-73`, Docs' `remember`
  at `index.rs:891-899`. Four copies of one `INSERT ... ON CONFLICT DO UPDATE`.
- `now_ms`. Docs `index.rs:905-910` and Mail `mirror/write.rs:42-46` are identical; Calendar's
  `store/write.rs:87-89` is the chrono equivalent.
- The placeholder helper: Docs `placeholders` (`index.rs:901-903`) and Mail `holes`
  (`mirror/read.rs:898-900`), same one-liner, different name and separator.
- The FTS5 tokenizer string `unicode61 remove_diacritics 2`, in Docs `index.rs:119-122` and Mail
  `mirror/mirror.sql:169-178`.
- `.map_err(|e| e.to_string())`, 309 occurrences (Calendar 59, Docs 36, Mail 214). Not a function
  waiting to be extracted; a `From` impl waiting to be written.
- The migrate skeleton: read version, refuse if newer, return if equal, run the ladder, stamp.

**Where they legitimately differ.** Three connection ownership models, each correct for its app:
`Mutex<Connection>` (Calendar `store/mod.rs:17`), `Mutex<Option<Connection>>` behind a writer thread
and a `OnceLock<Sender>` (Docs `index.rs:143-151`), and `Mutex<HashMap<String, Connection>>` with one
pair of ATTACHed files per account (Mail `db.rs:31-38`). Pragmas are the same three ideas delivered
three ways: `pragma_update` calls (Calendar `store/mod.rs:17-34`), an `execute_batch` literal (Docs
`index.rs:208-221`), an `execute_batch` with a formatted ATTACH (Mail `db.rs:129-151`). Version
storage differs on a real decision: `PRAGMA user_version` in Docs (`index.rs:229-248`) against a
`meta` row in Calendar and Mail. FTS5 is in two apps and everything above the tokenizer line
differs: Docs ranks with `bm25` and `highlight` (`index.rs:1184-1188`), Mail uses the index purely
as a membership subquery (`mirror/read.rs:234`) under a query language with `from:` and `has:`
operators (`mirror/fts.rs:100-127`).

**Two defects.** Mail has no transactions outside its two migrations: grepping the whole crate for
`unchecked_transaction`, `BEGIN IMMEDIATE`, `.transaction()` and `SAVEPOINT` returns exactly three
hits, two `execute_batch("BEGIN;")` in `mirror/schema.rs:37` and `state/schema.rs:33`, and one
SAVEPOINT in `state/journal.rs:278`. So `apply_and_queue` (`mirror/mod.rs:167-205`), which does N
flag updates plus N outbox inserts, runs unwrapped. Separately, Calendar's migration ladder is
`if found < 1 { V1 } else if found < 2 { V2 }` (`store/schema.rs:115-120`), an `else if`, which will
not compose when V3 lands. Mail's sequential `if`s (`mirror/schema.rs:39-44`) will.

Neither app uses `prepare_cached` anywhere. Mail's `still_bodiless` (`mirror/read.rs:949-969`)
prepares inside a loop.

**The crate.** `margin-sqlite`, roughly 250 lines: `open(path, pragmas)`, `Tx` and `Savepoint` RAII
guards (Calendar's `store/write.rs:62-85` is the right shape already and takes `&Connection` rather
than `&mut`, which is exactly what Mail needs from inside `Db::with`), `migrate(conn, &[&str],
version_store, noun)`, `meta_get`/`meta_set` generic over the table name so Mail's `state.meta`
works, `holes(n)`, `now_ms`, and a `From<rusqlite::Error>` error type. Net deletion is maybe 150
lines. Do it for the two defects it fixes and for the busy timeout, which only Mail sets today
(`db.rs:129-151`, 5s), not for the volume. Every schema, every read and every write stays per app.

## Logging

Only Margin Mail logs. `src-tauri/src/log.rs` is 137 lines, 94 of them not tests.

The surface: `CAP_BYTES = 256 * 1024` (`log.rs:23`), `init(&Path)` (`:34`) which sets a
`OnceLock<PathBuf>` to `dir.join("margin-mail.log")`, `path()` (`:38`), `note(who, line)` (`:45`)
which always `eprintln!`s and then, only if `PATH` is set, timestamps and appends under a
process-wide `Mutex<()>`, `trim` (`:55`) which flattens newlines and cuts at 2,000 characters, and
`append` (`:65`) which is not rotation but a keep-the-newest-half rewrite when the cap is exceeded.
`#[tauri::command] log_note` (`:90`) is registered at `lib.rs:361`, and the webview is the heavier
producer: `src/ipc.ts:752-758` logs every rejected `invoke`, and `src/main.tsx:21,24` catch
`window.onerror` and `unhandledrejection`.

The design decision worth keeping: `log.rs` is **told** its directory rather than reaching for an
`AppHandle`. `db.rs:46` calls `log::init(&data)` from inside `Db::open`, so the engine still works
under `cargo test`. Before that call, lines go to stderr, which under a Finder launch is nowhere.

The other three:

| | `eprintln!` | `println!` | log crate | tracing | plugin-log | log file |
|---|---|---|---|---|---|---|
| Margin | 0 | 0 | no | no | no | none |
| Calendar | 0 | 0 | no | no | no | none |
| Docs | 6 | 0 | no | no | no | none |
| Mail | 1 (inside `log.rs`) | 3 (dead) | own module | no | no | `margin-mail.log` |

Docs' six are `lib.rs:270`, `lib.rs:278`, `index.rs:437`, `index.rs:596`, `watch.rs:210` and
`writingtools.rs:136`, all going to `/dev/null` under a Finder launch. Margin and Calendar have
nothing at all: when a Drive backup or a calendar sync fails, the string reaches the frontend and
then the process forgets it.

**This is the clearest shared-crate win in the whole audit.** `margin-log` is `init`, `note`, `path`
and the `log_note` command, about 100 lines, dependent only on `chrono` and `std`, with one thing to
parameterise (the filename, derivable from the bundle identifier). Three apps gain the ability to
answer "why did it fail" after the process has exited, which is the standing rule for this suite.
Two things to fix while lifting: `note` discards the result of `append`, so a failed write is
invisible, and it does blocking file I/O under a `std::sync::Mutex` from async contexts
(`sync/engine.rs:327`, `:363`, `:448`, `sync/hydrate.rs:276`, `google/gmail.rs:106`). Bounded and
infrequent, so not urgent, but do not copy it into three more apps unexamined. The frontend half
(the `.catch` in `ipc.ts` plus the two handlers in `main.tsx`) is 15 more lines per app and catches
most real failures.

## Settings and persisted state

Only Mail has a settings layer in Rust. `settings.rs` is 479 lines (343 non-test): `settings.json`
in the app data dir, a 25-field `Settings` struct at `dto.rs:768`, a hand-written `defaults()`
(`settings.rs:32`), a genuine recursive JSON merge for `settings_set(patch)` (`merge_into`,
`settings.rs:234`), an atomic write through `library::atomic_write` (`settings.rs:221`), and a
deliberate refusal to reset on a malformed file (`settings.rs:210`, tested at `:430`). Mail also
owns `accounts.json`, `keymap.json` and `imap-trust.json` in the same directory.

Where everyone else keeps configuration:

- **Margin**: `localStorage`, 16 keys. Per-project settings live inside the `.margin` book file,
  merged against TypeScript defaults at `src/model/book.ts:212`. Rust holds no settings; its one
  JSON file is `backup.json` (`gdrive.rs:139`), Drive bookkeeping.
- **Calendar**: `localStorage`, five keys. Its entire Settings screen edits one preference, week
  start day. No config file on disk in any format, and no `atomic_write` in the crate.
- **Docs**: `localStorage`, seventeen keys behind zustand stores. Rust owns `roots.json`
  (`fs.rs:60,778`), which is workspace state rather than settings, and the index database.

**Not a crate.** Lifting `settings.rs` means inventing a settings backend for three apps that do not
have one and whose preferences currently live in the webview. That is a feature, not a refactor, and
the 25-field struct cannot move regardless. If it is ever wanted, the reusable core is: read JSON,
deep-merge a patch, atomic write, error rather than reset on a parse failure. About 60 lines.

One latent bug to fix in place: `Settings` has exactly one `#[serde(default)]` field
(`dto.rs:805`, `notifications`). Every other field is required, so the next field added without one
will fail to parse every existing install's `settings.json` and `settings_get` will error out. There
is a regression test for the one field that has a default (`settings.rs:389`), but the pattern was
not generalised.

## Filesystem helpers

Four `atomic_write`s, three genuinely different algorithms, all of them `write, fsync, rename` and
**none of them fsyncing the parent directory**, so on all four a crash can still lose the rename.

- **Docs** `fs.rs:311-347` with helpers at `:222-288`. A per-path `Arc<Mutex<()>>` lock map so a
  debounced autosave cannot race Cmd+S; a copy of the original into the temp before truncating so
  macOS ACLs, Finder tags and the exec bit survive; a hidden collision-retried temp name
  (`.{name}.{pid}-{seq}-{nanos:x}.tmp`, 64 tries); four `watch::note_self_write` calls; and
  `remove_file` on both error paths.
- **Margin** `project.rs:13-30`. Adds `.bak` rotation. When `backup` is false it `remove_file`s the
  target before the rename (`project.rs:26`), opening a window where the file does not exist.
  Leaves the temp behind on failure.
- **Mail** `library.rs:19-33`. Adds `create_dir_all`. Uses `with_extension`, which for a path with
  no extension produces a doubled dot. Leaves the temp behind on failure. No lock.
- **`write_private`** in Calendar `google/secrets.rs:243-261` and Mail `google/secrets.rs:245-263`
  is a fourth variant and the only byte-identical pair, `0o600`. That one belongs to the Google note.

**Do not share the general one.** Sharing it either drops Docs' watcher integration and lock map or
drags the file watcher into the shared crate. Each divergence is justified in a comment in its own
file. Do fix the two real bugs listed above, in place.

Not everything even goes through it: Mail writes the mbox export straight to `fs::File::create`
(`exports.rs:99-104`) and the attachment cache with plain `fs::write` (`attachments.rs:279`, `:482`).

**Trash**: Docs only. `trash = "5"`, used at `fs.rs:712-730` with `DeleteMethod::NsFileManager` on
macOS chosen deliberately over the crate default to avoid an Apple event entitlement, and used again
as the safe half of a cross-volume move (`fs.rs:676-677`). Margin's `delete_book` (`library.rs:143`)
is a bare `remove_file`.

**Path validation**: Docs is the only app with a real gate. `resolve` rejects non-absolute paths and
any `Component::ParentDir`, canonicalises the deepest existing ancestor and re-appends the tail;
`resolve_in_roots` requires `starts_with` an open root; `checked` (`fs.rs:212-214`) is what every
path-taking command calls, reads included. `check_name` (`fs.rs:149-158`) rejects empty, `.`, `..`,
separators and NUL. Mail sidesteps the problem by never letting the frontend name a write target;
its only sanitiser is `free_path` (`attachments.rs:363-389`), which maps separators to `-` and
trims dots. **Margin has none**: `project.rs:32-46` exposes `read_file`, `write_file` and
`write_bytes` as commands taking an arbitrary absolute path from the webview with no checking at
all. Its one validated path is the book id whitelist at `library.rs:61-66`. That is a finding for
Margin, not an argument for a crate.

**File watching**: Docs only. `notify 8`, `notify-debouncer-full 0.7` and `ignore 0.4` appear in no
other app. 300ms debounce (`watch.rs:33`), `NoCache` chosen over the file-id cache because on macOS
the inode cache folds the two halves of a rename together (`watch.rs:222-229`), self-write
suppression on a 2s window (`watch.rs:55,70-71`), one emit per event (`watch.rs:147`), and `kind`
derived from a fresh `symlink_metadata` rather than trusted from FSEvents flags
(`watch.rs:402-429`). One app watches files. There is nothing to share.

## Error types

Already uniform, and there is nothing to fix. **Every one of the 165 `#[tauri::command]`s across the
four apps returns either a bare value or `Result<T, String>`, with zero exceptions** (Margin 24,
Calendar 13, Docs 38, Mail 90). Counts of `-> Result<T, String>` anywhere: 52, 94, 91, 485.

`thiserror` and `anyhow` are dependencies of none of the four. Every `Display` is hand-written. The
custom enums are internal and never cross to the frontend: Calendar `ApiError` (`google/api.rs:132`,
four variants), Mail `ApiError` (`google/api.rs:184`, seven), Mail `ProviderError`
(`provider/mod.rs:26`), Mail `Refused` (`imap/tls.rs:95`). There are five `impl From` in the whole
suite. The two `ApiError`s look like the same type and are not: Calendar needs `SyncTokenExpired`
and `PreconditionFailed`, Mail needs `Unauthorized`, `InsufficientScope` and `Dropped`, and even the
shared four-line `From<reqwest::Error>` differs deliberately, with a comment in Mail explaining why
Calendar's simpler classification would be wrong for a mail client waking from sleep.

A shared `Result`/`Error` shape would be churn. The one useful piece is the
`From<rusqlite::Error>` that kills 309 `.map_err(|e| e.to_string())`, and that lives in the SQLite
crate.

## Updates

Margin's `updates.rs` is 90 lines and does three things: derive a channel from the merged plugin
config plus a Mac App Store receipt probe, query Apple's lookup endpoint for a newer App Store
version, and open `macappstore://`. **It is almost entirely app-agnostic already.** `channel()`
reads `handle.config().plugins.0` for `"updater"` and `"appstore"`; `mas_receipt()` walks
`current_exe()` up two levels to `_MASReceipt/receipt`; `appstore_latest()` reads
`config().identifier` and `package_info().version`. No product name, no bundle id, no endpoint is
hardcoded. It would drop into any of the other three unchanged.

| | Margin | Calendar | Docs | Mail |
|---|---|---|---|---|
| `tauri-plugin-updater` | yes | yes | yes | yes |
| Conditional registration | `lib.rs:159` | `lib.rs:260` | `lib.rs:263` | `lib.rs:265` |
| Channel concept | yes | no | no | no |
| App Store vs direct split | yes | no | no | no |
| `packaged_by` | no | `lib.rs:239` | no | `lib.rs:196` |
| Release pubkey | real | real | **placeholder** | **placeholder** |

Docs and Mail both ship a literal `REPLACE_WITH_...` string as the updater pubkey in
`tauri.release.conf.json`, and neither release workflow substitutes it (the workflows only set
`TAURI_SIGNING_PRIVATE_KEY`). Neither app can ship a verifiable direct-download update today. All
four release configs are 14 lines with an identical structure differing only in pubkey and repo slug.

**Share it.** `updates.rs` plus `packaged_by` is one 110-line module with one thing to parameterise,
and even the env var name could be derived from the bundle identifier. Second cheapest win after
logging.

## Async

Margin has **no tokio dependency at all**. Docs declares `tokio = { features = ["sync", "time"] }`
(`Cargo.toml:34`) and never uses it: grep for `tokio::` in its `src-tauri/src` returns nothing. That
line is a copy from a sibling and should go.

`tauri::async_runtime::spawn` is the house style in the three apps that spawn (Margin `gdrive.rs:765`,
Calendar `sync.rs:205` and five more, Mail `badge.rs:88` and five more). Raw `tokio::spawn` appears
only in Mail's IMAP autodiscovery fan-out (`imap/discover.rs:69-72`, `:382-383`, `:419`), which is
safe because Tauri's runtime is tokio but leaves those tasks untracked by Tauri's shutdown. Docs uses
no async runtime for background work at all: three `std::thread::spawn`s (`watch.rs:260`,
`watch.rs:287`, `index.rs:203`) and `#[tauri::command(async)]` on sync functions.

The two poll loops are the closest pair of non-trivial code in the suite and are still not the same.
Calendar `sync.rs:204-215` and Mail `sync/mod.rs:374-386` share the skeleton, share
`FIRST_PASS_SECS = 2`, and share a `focused()` helper that is character-for-character identical
(Calendar `sync.rs:218-223`, Mail `sync/mod.rs:388-392`). They differ on the wake: Calendar awaits a
`tokio::sync::Notify` with a timeout, so `kick` (`sync.rs:238-242`) can pull the next tick forward,
and it listens on `store-changed` to catch a freshly connected account (`sync.rs:197-202`). Mail's
is a bare `sleep`, and `kick` (`sync/mod.rs:456-464`) spawns a separate `sync_now` instead, relying
on the `running: AtomicBool` re-entrancy guard (`sync/engine.rs:74-84`) to keep the two from
overlapping. Both work. They are two answers, not one shared answer.

Nobody uses `tokio::time::interval`, `CancellationToken`, `watch::channel`, `parking_lot` or
`RwLock`. Cancellation, where it exists, is a re-entrancy guard (Mail's `AtomicBool`, Calendar's
`tokio::sync::Mutex<()>` with `try_lock`), a dropped stream (Mail `attachments.rs:140-160`), or a
`Weak` (Docs `watch.rs:200,262`). The three debounce helpers (Docs `watch.rs:376-387` and
`index.rs:451-464`, Mail `badge.rs:79-92`) are three different things; there is nothing to lift.

Events: all four use the `Emitter` trait and `app.emit(name, payload)` broadcast. **Nothing anywhere
uses `emit_to` or `emit_filter`**, which is fine while every app is single-window. Names are
kebab-case and overlap heavily: `menu-action` in all four, `store-changed`, `sync-progress` and
`auth` in Calendar and Mail, `pdf-warnings` in Margin and Docs. Docs is the only app defining them
as constants on both sides (`index.rs:42`, `watch.rs:26`, `src/ipc.ts:43-52`).

The lock rule is applied consistently and is worth writing down as a guideline rather than a crate:
`std::sync::Mutex` for SQLite behind a `with(|conn| ...)` closure, `tokio::sync::Mutex` for anything
held across an `.await`. Stated explicitly at Calendar `sync.rs:112-115` and Mail `db.rs:12-15`.

The `Sink` trait (Calendar `sync.rs:68-78`, Mail `sync/mod.rs:230-266`) is the one abstraction
arrived at twice independently: `status` and `changed` methods, an `AppSink { app: AppHandle }`
implementation, existing so a sync pass can be tested against a recorder. It is 12 lines and it
belongs with the sync engine, which is not shared. **No async crate.**

## Dependency drift

Agreed in all four and not worth a table row: `tauri` and `tauri-build` at 2, `serde` and
`serde_json` at 1, `base64` 0.22, `tauri-plugin-opener` 2, `objc2` 0.6 and `objc2-foundation` 0.3.
Also agreed where shared: `sha2` 0.10, `rand` 0.8, `url` 2, `chrono` 0.4, `tempfile` 3 (dev),
`harper-core` =2.5.0, `typst` and `typst-pdf` 0.14.2, `typst-as-lib` 0.15.5, `objc2-app-kit` 0.3,
`objc2-ui-kit` 0.3, `block2` 0.6, `tauri-plugin-dialog` and `tauri-plugin-deep-link` at 2.

Where they disagree, blank meaning the app does not have it:

| Crate | Margin | Calendar | Docs | Mail |
|---|---|---|---|---|
| rusqlite | | **0.37** | 0.40 | 0.40 |
| reqwest | 0.12 | 0.12 | | **0.13** |
| chacha20poly1305 | | **0.10** | | 0.11 |
| fontdb | 0.23 | | 0.23 | **0.24** |
| tokio | none | 1 (sync, time) | 1 (**unused**) | 1 (sync, time, net, io-util, rt) |
| tauri-plugin-process | 2 (**ungated**) | 2 (gated) | 2 (gated) | 2 (gated) |
| tauri-plugin-updater | 2 (**ungated**) | 2 (gated) | 2 (gated) | 2 (gated) |

Resolved in the lockfiles: `tauri` is 2.11.3 in Margin and 2.11.5 in the other three; `serde` 1.0.228
vs 1.0.229; `tokio` 1.52.3 vs 1.53.1; `libsqlite3-sys` 0.35.0 (Calendar) vs 0.38.2 (Docs, Mail);
`wry` 0.55.1 and `objc2` 0.6.4 everywhere. `reqwest` 0.12 and 0.13 are both in Margin's and
Calendar's graphs already.

Four real drifts to close: rusqlite 0.37 in Calendar against 0.40 elsewhere, reqwest 0.12 against
0.13 in Mail, chacha20poly1305 0.10 against 0.11, fontdb 0.23 against 0.24. The last three matter
because Calendar and Mail share a sealed-token format and Margin and Docs share a Typst pipeline; a
version split inside a pair that is meant to be the same code is how the two copies quietly stop
being the same code.

## What to build, and the one obstacle

Build three crates:

1. **`margin-log`**, about 100 lines. `init(dir, filename)`, `note(who, line)`, `path()`, the
   `log_note` command. Highest value: three apps currently cannot answer why anything failed.
2. **`margin-shell`**, about 250 lines. The builder prologue, the menu scaffold behind a `MenuSpec`,
   the `menu-action` forwarding, `show_main_window`, `hide_on_close`, `app_data_dir`,
   `packaged_by`, and Margin's `updates.rs` unchanged. Deletes roughly 400 lines and forces one
   answer to the close-button question that currently has three.
3. **`margin-sqlite`**, about 250 lines. `open` with pragmas including a busy timeout, `Tx` and
   `Savepoint`, `migrate`, `meta_get`/`meta_set`, `holes`, `now_ms`, an error type. Deletes maybe
   150 lines and fixes Mail's missing transactions and Calendar's non-composable ladder.

Do not build a settings crate, an error crate, a filesystem crate, an async crate or a DTO macro.
For each of those, either only one app has the thing, or all four already do the same trivial thing
in the same trivial way, or the apparent duplication dissolves on reading the divergences, every one
of which is justified in a comment where it sits.

The obstacle is mechanical and is the same one `repo-facts.md` describes for `margin-shared`. These
are four separate git repositories with no cargo workspace and no path dependencies between them,
and Margin Mail has no remote at all and one scaffold commit under 123 uncommitted files. A Cargo
path dependency walking out of one checkout into a sibling would fail on a fresh clone and in CI
exactly as the npm one already does. Decide where the shared crates live, a fifth repository
consumed by git tag or a monorepo, before writing a line of them.
