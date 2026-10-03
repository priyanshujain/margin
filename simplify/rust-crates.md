# Rust crates

The plumbing under the four apps that is genuinely the same code: the Tauri builder, the menu, the
window lifecycle, the updater wiring, a log file, and a thin layer over rusqlite. Three crates, not
ten. Every number below was measured on the four trees on 2026-09-06 and is recheckable against
[.research/rust-core.md](.research/rust-core.md). `margin-google`, `margin-secrets` and `margin-http`
belong to [accounts.md](accounts.md), `margin-typeset` and `margin-mac` to
[typesetting.md](typesetting.md); they appear here only where a dependency between crates matters.

## Where this departs from repo-layout.md

[repo-layout.md](repo-layout.md) named its crates before this audit existed, and where the two
disagree this document is the authority. Five changes, all narrowings except the last. `margin-db`
becomes **`margin-sqlite`** and drops the FTS5 helpers, because FTS5 is in two apps and everything
above the one shared tokenizer line differs: Docs ranks with `bm25` and `highlight`
(`index.rs:1184-1188`), Mail uses the index as a membership subquery (`mirror/read.rs:234`) under a
query language with `from:` and `has:` operators. **`margin-paths`** is not built: the five lines worth
sharing are `app_data_dir`, which moves into `margin-shell`, and the rest, atomic write, trash and
path validation, is three different algorithms with the divergence justified in a comment in each.
**`margin-ipc`** is not built: all 165 commands already return the same shape, so there is no error
type to unify. **`margin-update`** is a module inside `margin-shell` rather than a crate, because
Margin's `updates.rs` is 90 lines and has no consumer that does not also want the builder prologue
next to it. And **`margin-shell`** is new, has no entry in repo-layout.md, and is the largest single
win in the audit.

## 1. The headline

73 distinct lines appear verbatim, indentation included, in all four apps' `src-tauri/src/lib.rs`.

| | `lib.rs` lines | code lines | verbatim in all four | share of code |
|---|---|---|---|---|
| Margin | 256 | 237 | 114 | 48% |
| Margin Calendar | 354 | 279 | 123 | 44% |
| Margin Docs | 365 | 291 | 120 | 41% |
| Margin Mail | 475 | 386 | 124 | 32% |

That is 481 lines of one file written four times, 84 to 96 per app excluding brace-only lines, and two
copies say so: Calendar `lib.rs:248-249` and Mail `lib.rs:251-252` both carry the comment "Ported from
margin's lib.rs". The shell deletes roughly 400 of the 481, plus the four copies of `app_data_dir`
(Calendar's and Docs' `library.rs` are byte-identical nine-line files, Margin's copy is
`library.rs:49-53`, Mail's `library.rs:8-12`) and the second copies of `packaged_by` and
`show_main_window`; `margin-sqlite` takes another 150.

Six hundred lines out against 600 written once is not the argument. The argument is that the close
button has three answers, the schema ladder has two and one does not compose, the transaction boundary
is missing where it is most needed, and three apps cannot say why anything failed once the process has
exited.

## 2. The crates to build

### margin-shell

**What it owns.** The desktop plugin prologue, the menu scaffold and its event forwarding, the window
close policy and the Dock reopen, `app_data_dir`, `packaged_by`, and the update channel logic.

**The duplication.** The prologue is verbatim in all four (Margin `lib.rs:149-162`, Calendar
`250-263`, Docs `250-266`, Mail `253-268`). The menu scaffold, meaning `Menu::default(handle)`, the
`submenus` collect, the `find_submenu` closure, the `match find_submenu("File")` with its
`prepend_items` and `SubmenuBuilder` arms, the Edit and Help appends and the platform blocks, is Margin
`lib.rs:46-60` and `62-93`, Calendar `50-64` and `66-95`, Docs `100-114` and `115-146`, Mail `84-98`
and `100-135`; a pairwise diff of the whole `build_menu` puts Calendar against Mail at 49 differing
lines out of 118 and 123. The forwarding line is character-identical in all four:
`app.emit("menu-action", event.id().0.as_str()).ok();` (Margin `192`, Calendar `308`, Docs `318`, Mail
`341`). `show_main_window` is identical between Calendar `227-234` and Mail `184-191`, `packaged_by`
between Calendar `239-244` and Mail `196-201` but for the env var name. And Margin's `updates.rs`
hardcodes no product name, bundle id or endpoint, reading `handle.config().plugins.0`,
`config().identifier` and `package_info().version`, so it drops into the other three unchanged.

```rust
pub fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String>;

/// Process and updater plugins on desktop only, the updater only when the merged config carries an
/// `updater` key. Verbatim from all four today.
pub fn desktop_plugins<R: Runtime>(builder: Builder<R>, context: &Context<R>) -> Builder<R>;
pub enum Row<'a> { Item { id: &'a str, label: &'a str, accel: Option<&'a str> }, Separator }
pub enum AppSubmenu { TauriDefault, Trimmed }
pub enum Handled { Yes, No }
pub enum OnClose { HideWindow, DestroyAndKeepRunning, Quit }

pub struct MenuSpec<'a> {
    pub file: &'a [Row<'a>],          // prepended, or built fresh when there is no File submenu
    pub edit_append: &'a [Row<'a>],
    pub view_prepend: &'a [Row<'a>],
    pub window_append: &'a [Row<'a>],
    pub help_append: &'a [Row<'a>],
    pub check_updates: bool,          // suppressed when updates::channel() is "none"
    pub app_submenu: AppSubmenu,
}

pub fn standard_menu<R: Runtime>(handle: &AppHandle<R>, spec: &MenuSpec<'_>) -> tauri::Result<Menu<R>>;

/// Builds the menu and installs the forwarder in one call. Every id in the spec is emitted as
/// `menu-action` unless the hook claims it, which is what Margin needs for `show-window`
/// (`lib.rs:194-197`). The hook defaults to a function returning `Handled::No`.
pub fn with_menu<R: Runtime>(builder: Builder<R>, spec: &'static MenuSpec<'static>,
                             hook: fn(&AppHandle<R>, &str) -> Handled) -> Builder<R>;

pub fn on_close<R: Runtime>(builder: Builder<R>, policy: OnClose) -> Builder<R>;
pub fn run<R: Runtime>(app: App<R>, policy: OnClose);   // wraps app.run with the matching Reopen arm
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>);
pub fn packaged_by_env(identifier: &str) -> String;     // studio.margin.mail -> MARGIN_MAIL_PACKAGED_BY
#[tauri::command] pub fn packaged_by(app: AppHandle) -> Option<String>;

pub mod updates {
    pub fn channel<R: Runtime>(handle: &AppHandle<R>) -> &'static str;   // direct | appstore | none
    #[tauri::command] pub fn update_channel(app: AppHandle) -> &'static str;
    #[tauri::command] pub async fn appstore_latest(app: AppHandle) -> Result<Option<AppStoreRelease>, String>;
    #[tauri::command] pub fn open_appstore(track_id: u64) -> Result<(), String>;
}
```

Deriving the id list from the spec rather than repeating it in a `matches!` guard removes a class of
drift: an id can be built into the menu, left out of the guard, and silently do nothing. All four id
sets agree today, checked; Margin's one extra, `show-window`, is handled in Rust, which is why the
hook exists. `updates::appstore_latest` takes its client from `margin-http` rather than Margin's own
`LazyLock<reqwest::Client>` (`updates.rs:6`), since Margin's graph already resolves two reqwest majors.

**Two open decisions the crate forces.** `AppSubmenu::Trimmed` is Docs' rebuilt macOS app submenu
(`lib.rs:164-176` explains why: Services advertises a list the app cannot see or order, and the Hide
rows are a window state nobody reaches for through a menu). It is the better implementation and
[risks.md](risks.md) says extraction takes the better version, but adopting it costs Cmd+H in the
other three, so confirm before flipping them. `OnClose` has three variants because the apps have three
behaviours; the crate does not choose, it makes each app name one.

**What an app still supplies.** The menu contents, the `invoke_handler` list, every `manage` call, the
body of `setup`, deep link registration (Calendar and Mail), and `main.rs`. One caveat: the name the
frontend invokes is the bare function name even for a command defined in a dependency, so the shared
crates own five global names, `update_channel`, `appstore_latest`, `open_appstore`, `packaged_by` and
`log_note`.

### margin-log

**What it owns.** A single log file in the app data directory, capped, with one line per failure.

**The duplication.** There is none, and that is the point. Only Mail logs: `src-tauri/src/log.rs`, 137
lines, 94 not tests. Docs has six `eprintln!`s (`lib.rs:270`, `:278`, `index.rs:437`, `:596`,
`watch.rs:210`, `writingtools.rs:136`), all going nowhere under a Finder launch, and Margin and
Calendar have neither: when a Drive backup or a calendar sync fails the string reaches the frontend
and the process forgets it. Cheapest crate to write, largest change, because the standing rule for
this suite is that an error report starts by reading the log.

```rust
pub const CAP_BYTES: u64 = 256 * 1024;

/// Told its directory rather than reaching for an `AppHandle`. Mail calls this from `Db::open`
/// (`db.rs:46`) so the engine still works under `cargo test`; keep that property.
pub fn init(dir: &Path, file_name: &str);
pub fn path() -> Option<&'static Path>;

/// Always to stderr; to the file as well once `init` has run. `who` is an account id or an area.
pub fn note(who: &str, line: &str);
pub fn note_result(who: &str, line: &str) -> std::io::Result<()>;   // same, without dropping the error

#[tauri::command] pub fn log_note(who: String, line: String);
```

Lift Mail's implementation, `trim` at a 2,000 character line cap and the keep-the-newest-half rewrite
in `append` included, and fix two things on the way: `note` discards the result of `append`
(`log.rs:52`), so a failed write is invisible, and it does blocking file I/O under a
`std::sync::Mutex` from async call sites (`sync/engine.rs:327`, `:363`, `:448`,
`sync/hydrate.rs:276`, `google/gmail.rs:106`), which is bounded and infrequent but should not be
copied into three more apps unexamined.

**What an app supplies.** The file name, and the frontend half, 15 lines per app and the half that
catches most real failures: the `.catch` in the IPC wrapper that logs every rejected `invoke` (Mail
`src/ipc.ts:752-758`) and the `window.onerror` and `unhandledrejection` handlers (Mail
`src/main.tsx:20-26`), which belong in `@margin/ipc`. The crate depends on `chrono`, adding it to
Margin's and Docs' graphs, a small price for four logs stamped alike.

### margin-sqlite

**What it owns.** Opening a connection with the right pragmas, transaction and savepoint guards, the
migration runner, the meta table accessors, and an error type that converts.

**The duplication.** Three apps have SQLite, about 8,500 lines between them, of which 80 to 120 are
genuinely the same code. Do not build this for the volume. `version()` is byte-identical between
Calendar `store/schema.rs:130-136` and Mail `mirror/schema.rs:127-135`, and identical again modulo the
`state.` prefix at Mail `state/schema.rs:56-64`. The `meta` upsert, one `INSERT ... ON CONFLICT DO
UPDATE`, exists four times: Calendar `store/write.rs:214-228`, Mail `mirror/write.rs:57-73`, inside
both migrate functions, and as Docs' `remember` (`index.rs:891-899`). `now_ms` is identical between
Docs `index.rs:905-910` and Mail `mirror/write.rs:42-46`, with Calendar's chrono equivalent at
`store/write.rs:87-89`. The placeholder helper is one line under two names, `placeholders`
(`index.rs:901-903`) and `holes` (`mirror/read.rs:898-900`). And `.map_err(|e| e.to_string())` appears
309 times in the database code alone (Calendar 59, Docs 36, Mail 214), which is not a function waiting
to be extracted, it is a `From` impl waiting to be written.

```rust
pub struct Error(String);
impl From<rusqlite::Error> for Error;
impl From<Error> for String;          // so a command can keep returning Result<T, String>

/// Defaults to WAL, synchronous NORMAL, foreign keys on, no journal size limit (Docs sets one at
/// `index.rs:208-221`) and a 5s busy timeout, which only Mail sets today (`db.rs:129-151`).
pub struct Pragmas { pub wal: bool, pub synchronous: Synchronous, pub foreign_keys: bool,
                     pub journal_size_limit: Option<i64>, pub busy_timeout: Duration }
impl Default for Pragmas;

pub fn open(path: &Path, pragmas: &Pragmas) -> Result<Connection, Error>;

/// Takes `&Connection`, not `&mut`. Calendar's `store/write.rs:62-85` already has this shape and it
/// is exactly what Mail needs from inside `Db::with`, which hands out a shared reference. Drop
/// rolls back.
pub struct Tx<'a>;
impl<'a> Tx<'a> {
    pub fn begin(conn: &'a Connection) -> Result<Tx<'a>, Error>;   // BEGIN IMMEDIATE
    pub fn commit(self) -> Result<(), Error>;
}

pub struct Savepoint<'a>;             // begin(conn, name) / release, Drop rolls back to it

pub enum VersionStore {
    UserPragma,                                         // Docs
    MetaRow { table: &'static str, key: &'static str },  // Calendar, Mail, and Mail's `state.meta`
}

/// `steps[i]` runs when the stored version is below `i + 1`. Sequential, so V3 composes. Refuses a
/// database newer than `steps.len()`, naming `noun` in the message the way all three do today.
pub fn migrate(conn: &Connection, steps: &[&str], store: VersionStore, noun: &str) -> Result<(), Error>;

pub fn meta_get(conn: &Connection, table: &str, key: &str) -> Result<Option<String>, Error>;
pub fn meta_set(conn: &Connection, table: &str, key: &str, value: &str) -> Result<(), Error>;
pub fn holes(n: usize) -> String;     // "?,?,?"
pub fn now_ms() -> i64;               // SystemTime, not chrono: adds nothing to Margin's or Docs' graph
```

**What an app still supplies.** Every schema, every read, every write, and the connection ownership
model, three things each right for its app: `Mutex<Connection>` (Calendar `store/mod.rs:17`),
`Mutex<Option<Connection>>` behind a writer thread (Docs `index.rs:143-151`), and
`Mutex<HashMap<String, Connection>>` with one pair of ATTACHed files per account (Mail `db.rs:31-38`),
whose formatted ATTACH stays in Mail because ATTACH takes no bound parameter.

## 3. The crates not to build

**No settings crate.** Only Mail has settings in Rust: `settings.rs`, 479 lines, a recursive JSON
merge for patches (`merge_into`, `settings.rs:234`), an atomic write, and a deliberate refusal to
reset on a malformed file (`settings.rs:210`, tested at `:430`). The other three keep preferences in
`localStorage`, 16 keys in Margin, five in Calendar, seventeen in Docs; Calendar's whole Settings
screen edits one preference, week start day. Lifting this means inventing a settings backend for three
apps that do not have one, a feature rather than a refactor, and the 25-field struct cannot move
regardless. The reusable core, if wanted, is 60 lines.

**No error crate.** Every one of the 165 `#[tauri::command]`s across the four apps returns either a
bare value or `Result<T, String>`, with zero exceptions: Margin 24, Calendar 13, Docs 38, Mail 90.
Counts of `-> Result<T, String>` anywhere are 52, 94, 91 and 485, and neither `thiserror` nor `anyhow`
is a dependency of any of them. The custom enums are internal and never reach the frontend, and the
two that share a name are not the same type: Calendar's `ApiError` (`google/api.rs:132`) needs
`SyncTokenExpired` and `PreconditionFailed`, Mail's (`google/api.rs:184`) needs `Unauthorized`,
`InsufficientScope` and `Dropped`, and even their `From<reqwest::Error>` differs deliberately, with a
comment in Mail on why Calendar's classification would be wrong for a mail client waking from sleep.
The one useful piece, the `From<rusqlite::Error>` that kills 309 `.map_err`, lives in `margin-sqlite`.

**No filesystem crate.** Four `atomic_write`s, three genuinely different algorithms. Docs'
(`fs.rs:311-347`, helpers at `:222-288`) carries a per-path `Arc<Mutex<()>>` lock map so a debounced
autosave cannot race Cmd+S, copies the original into the temp so macOS ACLs, Finder tags and the exec
bit survive, uses a hidden collision-retried temp name, and makes four `watch::note_self_write` calls;
Margin's (`project.rs:13-30`) adds `.bak` rotation, Mail's (`library.rs:19-33`) adds `create_dir_all`.
Sharing one either drops Docs' watcher integration and lock map or drags the file watcher into the
shared crate. Path validation is the same story: Docs has the only real gate (`checked` at
`fs.rs:212-214`), Mail never lets the frontend name a write target, and Margin has none, which is a
defect in Margin rather than an argument for a crate. Trash is one app; file watching is one app.

**No async crate.** Margin has no tokio dependency at all, and Docs declares one at `Cargo.toml:34`
and never uses it: grep for `tokio::` in its `src-tauri/src` returns nothing.
`tauri::async_runtime::spawn` is the house style in the three apps that spawn. The two poll loops
(Calendar `sync.rs:204-215`, Mail `sync/mod.rs:374-386`) share a skeleton, `FIRST_PASS_SECS = 2` and a
character-identical `focused()`, then diverge on what matters: Calendar awaits a `tokio::sync::Notify`
with a timeout so `kick` can pull the next tick forward, Mail does a bare `sleep` and has `kick` spawn
a separate `sync_now` behind an `AtomicBool` guard. The one abstraction arrived at twice, the `Sink`
trait (Calendar `sync.rs:68-78`, Mail `sync/mod.rs:230-266`), is 12 lines and belongs with the sync
engine, which is not shared. The lock rule all three follow, `std::sync::Mutex` for SQLite behind a
`with(|conn| ...)` closure and `tokio::sync::Mutex` for anything held across an `.await`, goes in the
guidelines and is not code.

**No DTO macro.** One convention, held rigidly across 65 structs: `#[derive(Debug, Clone, Serialize,
Deserialize)]` with `#[serde(rename_all = "camelCase")]`, `#[serde(default)]` on patch fields,
`Option<T>` rather than a sentinel, `i64` epoch milliseconds for time, and a string field with the
legal values in a doc comment instead of an enum; `#[serde(rename = ...)]` appears exactly once in the
suite (Calendar `dto.rs:36`). A derive macro would save one line per struct and put a proc-macro crate
in four build graphs. What costs something is that each of those 65 structs has a hand-written
TypeScript interface in `src/ipc.ts` with nothing checking they agree, and the first 41 lines of
Calendar's and Docs' `ipc.ts` are byte identical: a codegen question, `ts-rs` or `tauri-specta`, for
the frontend plan.

## 4. The defects found on the way

Ranked by what a user loses. "Ride along" means the extraction fixes it; "before" means fix it in
place first, because the extraction would otherwise carry a broken line forward.

**1. Mail applies flags and queues the push without a transaction.** `apply_and_queue`
(`mirror/mod.rs:167-205`) runs `apply_flags` and `outbox::queue_flags` for N messages, per account,
unwrapped. Grepping the crate for `unchecked_transaction`, `BEGIN IMMEDIATE`, `.transaction()` and
`SAVEPOINT` returns three hits: two `execute_batch("BEGIN;")` in the migrations (`mirror/schema.rs:37`,
`state/schema.rs:33`) and one savepoint at `state/journal.rs:278`. A failure mid-loop leaves a flag
changed locally with no outbox row, so it never reaches the server, or the reverse. Every Mail user,
silently. **Before**, with Calendar's `Tx` shape, which the shared crate then replaces.

**2. Margin takes an arbitrary absolute path from the webview.** `project.rs:33`, `:38` and `:43`
expose `read_file`, `write_file` and `write_bytes` as commands taking a `String` path with no
validation at all; the only checked path in the crate is the book id whitelist at `library.rs:61-66`.
Script execution in the webview becomes arbitrary read and write with the app's privileges, and Margin
renders prose from files it did not write. Docs has the answer to copy, `checked` at `fs.rs:212-214`.
**Before**, and not blocked on any of this work.

**3. Docs and Mail ship a placeholder updater pubkey.** `src-tauri/tauri.release.conf.json:8` reads
`REPLACE_WITH_TAURI_SIGNER_PUBKEY` in Docs and `REPLACE_WITH_THE_MINISIGN_PUBLIC_KEY` in Mail, and
neither workflow substitutes it: both only set `TAURI_SIGNING_PRIVATE_KEY` (Docs `release.yml:203-204`,
Mail `:207-208`). Margin and Calendar have real keys. Every direct-download install of two apps is
stranded on the version it was downloaded at, with no verifiable update path. **Before**; a release
concern rather than a crate one, but `margin-shell` owning the channel logic is when it stops hiding.

**4. Mail's `Settings` has 25 fields and one `#[serde(default)]`.** `dto.rs:768-813`, the single
default at `:805` on `notifications`. Every other field is required, so the next field added without
one fails to parse every existing install's `settings.json` and `settings_get` (`settings.rs:144`)
errors out for good, the deliberate no-reset-on-malformed policy (`settings.rs:210`) keeping it that
way. There is a regression test for the one field that has a default (`settings.rs:389`); the pattern
was never generalised. Every Mail user, on the first upgrade after the mistake. **Before**:
`#[serde(default)]` on all 25 plus a `Default` impl backed by `defaults()` (`settings.rs:32`).

**5. Margin Mail cannot compile for mobile.** `#[cfg_attr(mobile, tauri::mobile_entry_point)]` sits at
`lib.rs:203`, directly above `attach_account`, not above `pub fn run()` at `lib.rs:250`. Separately,
`setup` calls `listen_for_redirects` (`lib.rs:307`), `stop_uikit_shrinking_the_viewport` (`:310`) and
`watch_for_the_consent_tab_closing` (`:314`) under `cfg(mobile)`, `cfg(target_os = "ios")` and
`cfg(target_os = "android")`, and none of the three is defined anywhere in the crate: grep returns the
call sites and nothing else. All three exist in Calendar (`lib.rs:150-184`, `186-212`, `214-226`) and
were meant to be ported with the rest, the iOS and Android dependency blocks being already in Mail's
`Cargo.toml`. Nobody is affected today because no mobile build runs; everybody is, the first day one
does. **Fix the attribute before**; the two webview helpers then **ride along** into `margin-shell`
and `listen_for_redirects` into `margin-google`.

**6. Four apps, three window close behaviours.** Calendar (`lib.rs:313-321`) and Mail (`:346-354`)
prevent the close and hide the window, then restore on `RunEvent::Reopen` (Calendar `:342-348`, Mail
`:463-469`). Margin lets the window be destroyed but calls `api.prevent_exit()` (`lib.rs:234`) and
rebuilds from config on Reopen (`open_main_window`, `:241-256`). Docs calls `.run(context)` at
`lib.rs:363` with no `RunEvent` closure and no `CloseRequested` handler anywhere in the crate, so
closing the window quits the app and unsaved state goes with it. **Rides along**: `OnClose` makes each
app name its policy, and Docs' is the one to reopen with the user.

**7. Calendar's migration ladder will not compose.** `store/schema.rs:115-120` is
`if found < 1 { V1 } else if found < 2 { V2 }`. An `else if`, so a database at version 0 when V3 lands
runs V1, stops, and gets stamped as current. Mail's sequential `if`s (`mirror/schema.rs:39-44`) are
correct. Latent and total: every Calendar user with an old database, the day a third migration ships.
**Rides along**: `margin_sqlite::migrate` runs every step below the target.

**8. Four smaller ones.** No `atomic_write` in the suite fsyncs the parent directory, so a crash can
still lose the rename; Margin's also `remove_file`s the target before renaming when `backup` is false
(`project.rs:26`), and Mail's `with_extension` (`library.rs:23-26`) doubles the dot for a path with no
extension. `note` discards the result of `append` (`log.rs:52`), so a failed log write is invisible:
fix that one before publishing `margin-log`. Margin's window permissions sit in
`capabilities/default.json:8-9` rather than `desktop.json`, so they apply on phones, and Docs alone
carries `core:window:allow-toggle-maximize`. And `prepare_cached` appears zero times in the three apps
with a database, `still_bodiless` (`mirror/read.rs:949-969`) preparing inside a loop.

## 5. Version drift

Agreed everywhere and not worth a row: `tauri` and `tauri-build` at 2, `serde` and `serde_json` at 1,
`base64` 0.22, `tauri-plugin-opener` 2, `objc2` 0.6, `objc2-foundation` 0.3, and where shared, `sha2`
0.10, `rand` 0.8, `url` 2, `chrono` 0.4, `tempfile` 3, `harper-core` =2.5.0, `typst` and `typst-pdf`
0.14.2, `typst-as-lib` 0.15.5, `tauri-plugin-dialog` and `tauri-plugin-deep-link` at 2. Where they
disagree, blank meaning the app does not have it:

| Crate | Margin | Calendar | Docs | Mail | Land on |
|---|---|---|---|---|---|
| rusqlite | | **0.37** | 0.40 | 0.40 | 0.40 |
| reqwest | **0.12** | **0.12** | | 0.13 | 0.13 |
| chacha20poly1305 | | **0.10** | | 0.11 | 0.11 |
| fontdb | **0.23** | | **0.23** | 0.24 | 0.24 |
| tokio | none | 1 (sync, time) | 1 (**unused**) | 1 (sync, time, net, io-util, rt) | drop from Docs |
| tauri-plugin-process | 2 (**ungated**) | 2 gated | 2 gated | 2 gated | gate in Margin |
| tauri-plugin-updater | 2 (**ungated**) | 2 gated | 2 gated | 2 gated | gate in Margin |

Resolved in the lockfiles: `tauri` 2.11.3 in Margin against 2.11.5 elsewhere, `serde` 1.0.228 against
1.0.229, `tokio` 1.52.3 against 1.53.1, `libsqlite3-sys` 0.35.0 in Calendar against 0.38.2 in Docs and
Mail, `reqwest` 0.13.1 in Mail against 0.13.4 elsewhere; `wry` 0.55.1 and `objc2` 0.6.4 are uniform.
Margin's, Calendar's and Mail's graphs each carry two reqwest majors already, 0.12.28 and 0.13.x,
because `tauri-plugin-updater` pulls 0.13 whatever the app declares, so moving the direct dependency
to 0.13 collapses that to one copy in three apps.

These matter beyond tidiness. rusqlite 0.37 against 0.40 is a hard blocker for `margin-sqlite`, since
one crate cannot compile against two rusqlite majors in one graph, so Calendar moves first. Calendar
and Mail share a sealed-token format, so chacha20poly1305 0.10 against 0.11 is a split inside a pair
meant to be one implementation, and Margin and Docs share a Typst pipeline, so fontdb matters the day
`margin-typeset` lands. The shared crates themselves land on `rusqlite` 0.40 with `bundled` (the
feature that brings FTS5, already commented in Docs' and Mail's manifests), `tauri` 2 with default
features off, `chrono` 0.4 in `margin-log` only, and no reqwest in `margin-shell` at all.

## 6. Four repos, no workspace, one with no remote

Margin has a remote and 149 commits. Calendar has a remote, 27 commits and a clean tree. Docs has a
remote, 8 commits and 129 uncommitted files. **Margin Mail has no remote at all**, one scaffold commit
and 123 uncommitted files. There is no cargo workspace spanning them and no path dependency between
them.

A cargo path dependency walking out of one checkout into a sibling would fail on a fresh clone and in
CI in exactly the way the npm one already does. That is not a prediction: Docs' `package.json:33` and
Mail's `package.json:27` both read `"margin-shared": "file:../../python/margin/shared"`, which resolves
on this machine because of the order things were created in and nowhere else. Writing
`margin-log = { path = "../../python/margin-shared/crates/margin-log" }` reproduces it with a
different tool. So the mechanism is a cargo git dependency on the fifth repo, pinned to a tag:

    margin-shell = { git = "https://github.com/priyanshujain/margin-shared", tag = "v0.3.0" }

Cargo needs no registry for this, which is the one place Rust has it easier than npm. Three
consequences. Each app's `Cargo.lock` records the resolved git rev and stays committed, so moving a
tag changes nothing until someone runs `cargo update -p margin-shell`: treat tags as immutable. Local
iteration goes through a `[patch]` section or a `paths` entry in `.cargo/config.toml`, and per
[risks.md](risks.md) that is a switch a developer turns on, never a value the repo ships. And the
shared repo's CI has to build all four apps against a candidate before a tag is cut, which needs all
four checkoutable from CI, which needs Mail to have a remote. Nothing starts until Docs' 129 files and
Mail's 123 files are committed and pushed; that is a precondition, not a precaution.

## 7. Per app, exactly what changes

**Step 0, the shared repo.** Create the three crates under `crates/` in the MIT-licensed
`margin-shared` repository. `margin-log` first: 100 lines, no dependents among the other two, and the
only one that gives three apps a capability they lack. Then `margin-sqlite`, then `margin-shell`, which
needs `margin-http` for the App Store probe and so lands after [accounts.md](accounts.md)'s first
crate. Tag `v0.3.0`.

**Margin Mail** first: it is the source of `margin-log`, the worst affected by the missing
transactions, and the one needing the mobile fix.

1. Give it a remote and commit the 123 files.
2. Fix `apply_and_queue` with a local `Tx`, put `#[serde(default)]` on all 25 `Settings` fields, move
   the attribute at `lib.rs:203` onto `pub fn run()` at `:250` (defects 1, 4, 5).
3. Take `margin-log`; delete `log.rs` but keep the `log::init(&data)` call in `Db::open` (`db.rs:46`).
4. Take `margin-sqlite`: replace both `version()`s, both `meta` upserts, `holes`, `now_ms` and the two
   migrate skeletons; keep the ATTACH at `db.rs:129-151`; convert the 214 `.map_err` to `?`.
5. Take `margin-shell`: delete `library.rs:8-12`, `show_main_window`, `packaged_by`, the prologue and
   `build_menu`, the last becoming a `MenuSpec` const. `OnClose::HideWindow`, which is today's
   behaviour. Register `margin_shell::updates::*` and put a real updater pubkey in the release config.
6. Port Calendar's three mobile helpers, or drop the three call sites until mobile is real.

**Margin Calendar** second: the only clean tree, and the only app that has to move a rusqlite major.

1. Bump `rusqlite` 0.37 to 0.40 and `chacha20poly1305` 0.10 to 0.11; run the store tests.
2. Take `margin-sqlite`: `Tx` and `Savepoint` leave `store/write.rs:62-85`, `version()`, `meta_get`,
   `meta_set` and `now_ms` are deleted, and `migrate` (`store/schema.rs:105-128`) takes a `&[&str]` so
   the `else if` at `:117` stops being a bug.
3. Take `margin-log`, initialise it where the store opens, and route through `note` the errors that
   currently only reach the frontend.
4. Take `margin-shell`: delete `library.rs`, `show_main_window`, `packaged_by`, the prologue and
   `build_menu`. `OnClose::HideWindow`. Register the updates commands, which Calendar lacks.

**Margin Docs** third.

1. Commit the 129 files. Drop the unused `tokio` line at `Cargo.toml:34`; bump `fontdb` to 0.24.
2. Put a real pubkey in `tauri.release.conf.json:8` and make the workflow assert it is not a
   placeholder.
3. Take `margin-log` and convert the six `eprintln!`s into `note` calls. Largest behavioural gain of
   the whole plan for Docs.
4. Take `margin-sqlite` with `VersionStore::UserPragma`, keeping the writer thread and the
   `journal_size_limit` pragma, which becomes a `Pragmas` field.
5. Take `margin-shell`. Docs' rebuilt macOS app submenu is what the crate adopts, so this is a
   deletion here and a change for the other three. Choose an `OnClose`: today it is `Quit` by omission.

**Margin** last: the most local history, the least to gain.

1. Fix `project.rs:33`, `:38` and `:43` with Docs' `checked`, and the remove-then-rename window at
   `project.rs:26`.
2. Move `tauri-plugin-process` and `tauri-plugin-updater` from `[dependencies]` (`Cargo.toml:25-26`)
   into the `cfg(not(android, ios))` target block, and the two window permissions from
   `capabilities/default.json:8-9` into `desktop.json`. Bump `reqwest` to 0.13.
3. Take `margin-log`; Margin currently logs nothing at all.
4. Take `margin-shell`: `updates.rs` moves out wholesale and comes back as a dependency,
   `app_data_dir` leaves `library.rs:49-53`, `build_menu` becomes a `MenuSpec` with the `show-window`
   hook, and `OnClose::DestroyAndKeepRunning` preserves today's behaviour. Add `packaged_by` and tell
   the Nix wrapper the variable is `MARGIN_APP_PACKAGED_BY`.
5. No `margin-sqlite`: Margin has no database.
