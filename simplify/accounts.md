# Accounts, OAuth, secrets, HTTP and backup

The plan for `margin-google`, `margin-secrets` and `margin-http`, what happens to the two backup
designs, and why the two sync engines stay where they are. Every claim carries a file and a line,
read off disk on 2026-09-06. The audit behind it is
[.research/rust-google-sync.md](.research/rust-google-sync.md); you should not need to open it.
Margin Docs is absent throughout: its `src-tauri/Cargo.toml` has no `reqwest`, no `chacha20poly1305`
and no Google anything, so it consumes none of these three crates.

## The two OAuth implementations are one implementation

Margin Mail's `google/auth.rs` says so in its first four lines: ported from Margin Calendar's
`google/auth.rs`, which took it from margin's `gdrive.rs`. It is a copy with edits, and the copy is
measurable. Counts ignore each file's `#[cfg(test)]` module; a calendar line counts as present when
the identical line exists in Mail's file.

| File | Calendar | Mail | Calendar lines verbatim in Mail |
|---|---|---|---|
| `src-tauri/src/google/auth.rs` | 898 | 1,145 | 815 (91%) |
| `src-tauri/src/google/browser.rs` | 420 | 419 | 416 (99%) |
| `src-tauri/src/google/secrets.rs` | 262 | 265 | 243 (93%) |
| `src-tauri/build.rs` | 29 | 29 | 29, `diff` prints nothing |

1,503 of the calendar's 1,609 non-test lines exist unchanged in Margin Mail. `browser.rs`, the whole
iOS `SFSafariViewController` and Android Custom Tab consent surface, differs in four lines and all
four are comment prose naming which app the sheet sits over. On top of that, `margin`'s `gdrive.rs`
is 953 lines of which roughly 330 are the same flow written independently and earlier: the same PKCE
(gdrive.rs:159-169), the same loopback listener (gdrive.rs:199-275, inline in one function rather
than four testable pieces), the same token endpoint calls (gdrive.rs:318-359).

Six things genuinely differ, and those six are the shared crate's entire configuration surface.

1. **Scopes.** The calendar has one string, `pub const SCOPES` (calendar auth.rs:28). Mail has a
   six-entry `BASE_SCOPES` (mail auth.rs:29-35), a `REQUIRED_SCOPE` an account is refused for
   (auth.rs:41), `scope_string` for extras (auth.rs:274), and `granted_scopes` / `missing_required`
   reading what Google granted back off the token response (auth.rs:292, 300); the calendar's
   `TokenResponse` has no `scope` field. Parameters: `base_scopes` and `required_scopes`, slices,
   the second allowed to be empty.
2. **Re-consent.** Mail has `grant()` (auth.rs:725) because Google offers installed apps no
   incremental authorization: picking up `calendar.events` to answer an invite costs the whole
   consent screen again. No parameter; the calendar never calls it.
3. **`login_hint` and `prompt`.** Mail's `auth_url` takes a hint and switches `prompt` between
   `consent` and `select_account consent` (auth.rs:607-639); the calendar always sends
   `select_account consent` (calendar auth.rs:519-530). No parameter; `Option<String>` on `connect`.
4. **Revocation.** The calendar's `revoke` returns nothing and its result is discarded (calendar
   auth.rs:479); Mail keeps it and reports that the account went from this device but Google could
   not be reached (mail auth.rs:584, 1043). No parameter; the crate returns the outcome.
5. **Where the account record lands.** A SQLite row through `store::write::upsert_account` (calendar
   store/write.rs:168-181), against `accounts.json` through `crate::accounts::upsert`, because in
   Mail every account owns its own database and the account list must be readable before any of them
   is open (mail accounts.rs:1-15). Parameter: a trait object, the only one needing real design.
6. **HTTP body building.** Mail hand-rolls `form_body` over `url::form_urlencoded` (auth.rs:502-511)
   where the calendar uses `.form()`. See step 6: a mistake, not a difference.

Two smaller ones belong to `margin-secrets`: the `SERVICE` and `KEY_CONTEXT` constants (calendar
secrets.rs:41, 45; mail secrets.rs:42, 46), and a `reference()` helper the calendar needs for its
`accounts.keychain_ref` column (calendar secrets.rs:60-62) which Mail dropped.

## Defects found on the way

**1. margin keeps a Google refresh token in plaintext.** `BackupState.refresh_token` is a plain field
(margin gdrive.rs:68), serialised with `serde_json::to_string_pretty` and written by `save_state`
(gdrive.rs:153-157) through `crate::project::atomic_write` (project.rs:13-29), which sets no file
mode, into `backup.json` (gdrive.rs:138-140). Same OAuth client and same grant that the other two
apps seal. The weakest store in the suite sets the suite's real security level, so sealing anything
is worth nothing until this is fixed. **Fix before the refactor**: it is small against today's code,
and it is the one finding here that is live exposure rather than untidiness.

**2. Margin Mail will not compile for mobile.** `src-tauri/src/lib.rs:307` calls
`listen_for_redirects(handle)` under `#[cfg(mobile)]`, and `grep -rn "fn listen_for_redirects"` over
the whole repository returns nothing. The call site was copied from the calendar and the definition
was not; the calendar has it at `margin-caledar/src-tauri/src/lib.rs:150-172`. Desktop builds are
unaffected, and the plugin is already registered (mail Cargo.toml:24, lib.rs:260), so the only thing
missing is the twenty-line function. **Fix before the refactor**: the mobile deep link is one of the
two paths the crate must expose and there is no way to know Mail's half works.

The rest ride along with the refactor rather than blocking it.

| # | Defect | Where | What breaks |
|---|---|---|---|
| 3 | The calendar has no rate limit handling. `error_for` maps 410 and 412 and sends everything else to `Other`; grepping the crate for `429`, `Retry-After` or `backoff` finds nothing | calendar api.rs:191-197, push.rs:29, 345, 378 | a 429 reaches `push::drain`, counts as a real attempt through `mark_attempt_failed`, and five of them retire the queued write permanently. A user's edit is dropped silently because Google was busy |
| 4 | margin's HTTP clients have no timeouts. The calendar's own header named this as fix number one (calendar auth.rs:3) and nobody applied it upstream | margin gdrive.rs:25, updates.rs:6 | a hung socket hangs a backup with no bound |
| 5 | margin bakes a build-machine path into the binary and reads it at runtime in preference to the compiled copy | margin gdrive.rs:22-23, 41-42 | the build machine's absolute path ships in every binary, a developer's on-disk file silently outranks what was compiled, and with `build.rs` two lines long a clone without credentials fails to compile rather than failing at connect. Fix with step 6 |
| 6 | `gdrive_disconnect` revokes the suite-wide grant with `let _ =` and says nothing | margin gdrive.rs:777-803 | Disconnect in the writing studio signs the person out of the calendar and the mail client too. Mail names that before offering the button (mail auth.rs:1026-1029) |
| 7 | `chacha20poly1305` 0.10 against 0.11, no on-disk format difference | calendar Cargo.toml:40, mail Cargo.toml:64 | nothing today; settled at 0.11 |
| 8 | `reqwest` 0.12 against 0.13 | margin Cargo.toml:39, calendar Cargo.toml:29, mail Cargo.toml:37-43 | nothing today; settled at 0.13, and step 6 says why the three hand-written helpers this supposedly forced are avoidable |

## `margin-google`

`crates/google`, on reqwest 0.13, depending on `margin-secrets` and `margin-http`. It owns the OAuth
flow and nothing above it: no Gmail types, no Calendar types. The app hands it a config and a place
to record accounts, and gets back a token getter.

```rust
pub struct Config {
    pub base_scopes: &'static [&'static str],
    /// An account that did not grant all of these is refused, not stored. May be empty.
    pub required_scopes: &'static [&'static str],
    /// For the listener page and the not-set-up sentence.
    pub app_name: &'static str,
    /// e.g. "studio.margin.mail:/oauth2redirect".
    pub android_redirect: &'static str,
    /// `include_str!(concat!(env!("OUT_DIR"), "/google-credentials.json"))` from the app.
    pub credentials_json: &'static str,
}

/// One app writes a SQLite row and the other a JSON file; neither shape belongs in this crate.
pub trait AccountSink: Send + Sync {
    fn upsert(&self, app: &tauri::AppHandle, account: &Granted) -> Result<(), String>;
    fn forget(&self, app: &tauri::AppHandle, account_id: &str) -> Result<(), String>;
    fn email_of(&self, app: &tauri::AppHandle, account_id: &str) -> Result<Option<String>, String>;
    /// (account_id, email) for every account with a token, read once at launch.
    fn known(&self, app: &tauri::AppHandle) -> Result<Vec<(String, String)>, String>;
}

pub struct Granted {
    pub account_id: String,
    pub email: String,
    pub display_name: String,
    pub scopes: Vec<String>,
    /// Non-empty means nothing was stored and there is no account.
    pub missing_required: Vec<String>,
}

/// Managed in Tauri state, replacing both apps' own (calendar auth.rs:214-219, mail auth.rs:239-243).
pub struct AuthState { /* sessions, pending, config, sink, secrets */ }
impl AuthState {
    pub fn new(config: Config, sink: Arc<dyn AccountSink>, secrets: margin_secrets::Store) -> Self;
}

/// Returns the consent URL; the answer arrives later as the `auth` event. `hint` is the address
/// typed on the connect screen, when there was one.
pub async fn connect(app: tauri::AppHandle, extra_scopes: Vec<String>, hint: Option<String>)
    -> Result<String, String>;
/// The whole consent again for a connected account, to pick up a scope it did not grant.
pub async fn grant(app: tauri::AppHandle, account_id: String, extra_scopes: Vec<String>)
    -> Result<String, String>;
/// Revokes at Google, then forgets the token and the session here. The account goes from this
/// device whether or not Google answered; the return says whether it heard.
pub async fn disconnect(app: &tauri::AppHandle, state: &AuthState, account_id: &str)
    -> Result<Revoked, String>;
pub enum Revoked { AtGoogle, LocallyOnly(String) }
/// A live access token, refreshing if needed. Single-flight.
pub async fn valid_access_token(state: &AuthState, account_id: &str) -> Result<String, String>;
/// From `setup`, before any command can run. Seeds the session map with stored emails.
pub fn init_sessions(app: &tauri::AppHandle, data_dir: PathBuf);

#[cfg(mobile)] pub fn listen_for_redirects(handle: &tauri::AppHandle);
#[cfg(mobile)] pub async fn handle_redirect(app: tauri::AppHandle, incoming: &url::Url);
#[cfg(mobile)] pub async fn abandon_pending(app: tauri::AppHandle, reason: Option<String>);
```

**Desktop, and phones by default.** A loopback listener: bind 127.0.0.1 on a port the OS picks, open
the URL in the system browser through `tauri-plugin-opener`, never an in-app webview, and wait
`AUTH_TIMEOUT_SECS` (120 desktop, 900 mobile). It keeps the four testable pieces the calendar split
it into, `write_http_message`, `request_path`, `parse_redirect` and `await_code`, with the `Redirect`
enum, the CSRF state check, the `access_denied` case, the favicon skip, the 8 KiB buffer, the 150 ms
poll and the 5 s read timeout (mail auth.rs:307-437). Phones use it too: a Desktop client may
redirect to loopback on any port without registering it, and Google's token endpoint checks the
client id, the secret and the redirect rather than the calling OS. What used to make that impossible
on a phone was that leaving for Safari suspends the process, which `browser.rs` fixes by keeping the
consent sheet in front of the app rather than replacing it.

**Mobile deep link.** Runs instead when the credentials file carries a client for this platform,
setting `Credentials::platform_client` at load time (mail auth.rs:118-122). With no listener the
verifier goes in `Pending { state, verifier, redirect, expires }` behind a mutex with a 900 s expiry.
`handle_redirect` takes the verifier rather than reading it, so both arrival routes fire harmlessly:
`get_current` for the link that launched a process the OS had killed, `on_open_url` for the usual
case. Android uses `Config::android_redirect`, iOS the reversed client id, worth having there as the
only route to `ASWebAuthenticationSession`, the only iOS browser that shares Safari's cookies.

**Refresh and clock skew.** One constant, `EXPIRY_SKEW_SECS = 60`, with expiry stored as
`now() + expires_in.saturating_sub(EXPIRY_SKEW_SECS)` (mail auth.rs:61, 1122). That is all either app
does and it is enough: it covers a request in flight when the clock rolls over, and does not pretend
to fix a machine whose wall clock is wrong. Refresh is single-flight by holding the tokio mutex
across the refresh await, so concurrent callers queue on one token request; that serialises refreshes
across accounts too, the right trade for something happening once an hour per account. A rotated
refresh token is written back only when it changed (mail auth.rs:1114-1118). margin has none of this:
it reads the session, drops the lock, then awaits (gdrive.rs:498-521).

**Revocation** is `POST https://oauth2.googleapis.com/revoke` with the refresh token, before anything
local is touched, because the token is what names the grant. One OAuth client covers the suite and
the endpoint acts on the authorization behind the token rather than on the string, so this signs the
person out of every Margin app on every machine; the crate returns `Revoked` so the app can say so.

**The multi-account store** is `HashMap<String, Session>` keyed on the Google `sub` from the
id_token, falling back to the email when it is absent. `Session` holds the access token, its expiry
and the email; the refresh token is never in that map and never in memory outside a refresh. The
id_token is parsed and never signature-verified, deliberately: it arrived over TLS from Google's own
token endpoint, the same trust the access token rides on. The crate emits the `auth` event on Mail's
shape (`accountId`, `email`, `scopes`, `missingRequired`, `error`), a superset of the calendar's that
costs it two empty arrays; two schemas for one event name costs more.

## `margin-secrets`

`crates/secrets`. A sealed key-value file, not an OAuth token store. Mail already uses it for three
unrelated things: refresh tokens keyed by account id, the backup key under `"margin-mail backup key"`
(backup/crypto.rs:41, 119-138), and the R2 credentials under `"margin-mail r2 credentials"`
(backup/r2.rs:29). Neither of those ids can collide with an account id, because a Google `sub` is
digits and an address cannot carry a space.

```rust
pub struct Store { /* dir, context */ }
impl Store {
    /// `context` is a per-app constant mixed into the key, e.g. "margin-mail token store v1".
    pub fn new(dir: PathBuf, context: &'static str) -> Self;
    pub fn put(&self, id: &str, secret: &[u8]) -> Result<(), String>;
    pub fn get(&self, id: &str) -> Result<Option<Vec<u8>>, String>;
    pub fn delete(&self, id: &str) -> Result<(), String>;
    pub fn put_str(&self, id: &str, secret: &str) -> Result<(), String>;
    pub fn get_str(&self, id: &str) -> Result<Option<String>, String>;
}
```

A value, not the `OnceLock<PathBuf>` global both apps have today (mail secrets.rs:49-55), which
exists only because the free functions deliberately do not carry an `AppHandle`; a `Store` held by
`AuthState` gets the same property without process-wide state.

**The format** is unchanged from what is on disk, so there is no migration. `tokens.enc` is JSON, a
`BTreeMap<String, String>` of id to base64 (standard alphabet, padded) of `nonce || ciphertext ||
tag`, the nonce 24 random bytes, XChaCha20-Poly1305 throughout. One seal per entry rather than one
over the map, so a corrupt entry costs that entry. `tokens.salt` is 32 random bytes, per install,
written once and never rotated: losing it costs a reconnect and nothing else.

**The key** is `SHA256(context || salt || machine_id)`, never persisted. `machine_id` is
`/etc/machine-id` on Linux, `IOPlatformUUID` scraped out of `/usr/sbin/ioreg` on macOS, and
deliberately empty on iOS and Android, where the sandbox is the real boundary and a reinstall would
rotate any identifier and silently destroy the store. The salt sits beside the ciphertext and the
context is a constant in a public binary, so the machine id is the only thing binding a token to the
machine that stored it: a copied-home-directory defence and nothing stronger, and the file should
keep saying so. A salt that exists and will not read is refused rather than replaced, because
replacing it turns one transient IO failure into permanent loss of every token (mail
secrets.rs:107-113). No keyring, because macOS ties a keychain item's ACL to the code signature so
every ad-hoc rebuild re-prompts, and the `keyring` crate has no Android backend at all.

**Atomic replace.** Write to `<path>.tmp` opened with `mode(0o600)` on unix so the ciphertext is
never briefly world readable, `write_all`, `sync_all`, then `rename` (mail secrets.rs:246-265).
Deliberately not the apps' general `atomic_write`, which sets no mode. One hardening to fold in:
neither copy fsyncs the parent directory after the rename.

**The version drift** settles at `chacha20poly1305 = "0.11"`, which Mail already runs. 0.11 moved to
`hybrid-array`: `Key::from_slice` and `XNonce::from_slice` are deprecated and the array conversions
carry the length in the type, so the one panic those calls had is now a compile error. The on-disk
format is identical, so the calendar's upgrade is an API edit with no data migration.

## `margin-http`

`crates/http`, lifted wholesale from Margin Mail's `google/api.rs`, the only place in the suite where
any of this exists. There are nine `reqwest::Client` constructions across the three apps, no two
alike, two of them with no timeouts.

```rust
/// Named profiles rather than a builder, so a new call site picks one rather than inventing one.
pub fn auth() -> &'static reqwest::Client;      // connect 10s, total 30s, h2 + tcp keepalive
pub fn api() -> &'static reqwest::Client;       // connect 10s, total 60s, pool idle 30s, gzip, http2
pub fn bulk() -> &'static reqwest::Client;      // connect 10s, total 120s, uploads and downloads
pub fn untrusted() -> &'static reqwest::Client; // connect 5s, total 10s, referer(false), 3 redirects

pub enum ApiError {
    Unauthorized(String),
    InsufficientScope(String),
    RateLimited { retry_after_ms: u64 },
    NotFound(String),
    Offline(String),
    /// There and then not: reset, closed early, cut off mid-body. Its own kind because it is the
    /// one failure worth retrying at once, and the one a client that never does shows on a wake.
    Dropped(String),
    Other(String),
}

pub fn error_for(status: u16, context: &str, scope: &str, retry_after_ms: Option<u64>, body: &str)
    -> ApiError;
pub fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64>;
pub fn strip_urls(text: &str) -> String;
pub async fn read_json<T: DeserializeOwned>(resp: reqwest::Response, context: &str, scope: &str)
    -> Result<T, ApiError>;

pub const MAX_BACKOFF_MS: u64 = 64_000;
pub const MAX_ATTEMPTS: u32 = 5;
pub const DROPPED_WAITS_MS: [u64; 2] = [250, 1_250];
pub fn backoff_ms(attempt: u32, jitter_ms: u64) -> u64;

pub async fn with_retry<T, F, Fut>(call: F) -> Result<T, ApiError>
where F: FnMut() -> Fut, Fut: Future<Output = Result<T, ApiError>>;
/// For a call that must not be made twice: a send on a connection that dropped may have sent.
pub async fn with_retry_no_replay<T, F, Fut>(call: F) -> Result<T, ApiError>
where F: FnMut() -> Fut, Fut: Future<Output = Result<T, ApiError>>;

/// A rolling window of spend. Budget is a constructor argument: Gmail's 6,000 units a minute is not
/// the Calendar API's limit, and the per-call unit table stays in the app that knows it.
pub struct Quota { /* VecDeque<(at_ms, units)> */ }
impl Quota {
    pub fn new(budget: u32, window_ms: u64) -> Self;
    pub fn spent(&mut self, now_ms: u64) -> u32;
    /// How long before `units` more would fit. Zero when they fit now.
    pub fn wait_for(&mut self, now_ms: u64, units: u32) -> u64;
    pub fn charge(&mut self, now_ms: u64, units: u32);
}
```

`error_for` reads Google's machine-readable reason out of all three places Google puts it,
`error.status`, `error.errors[].reason` and `error.details[].reason`, because a 403 for an
insufficient scope only says so in the third (mail api.rs:250-275, 341-440). `dailyLimitExceeded` is
deliberately not retryable: it is the project's day gone, and retrying in thirty seconds only spends
the next day's. `strip_urls` drops whole sentences carrying a link, rather than the bare URL, because
"Enable it by visiting then retry" is not English. Backoff is `min(2^n seconds + jitter, 64s)`, with
jitter a parameter so the schedule is a pure function and testable. Set a user agent,
`Margin<App>/<version>`, on every profile: today nothing does except `mail/imap/discover.rs:543-554`,
which spoofs Chrome, correct for autodiscovery and wrong everywhere else, so `untrusted()` takes an
override. margin gains timeouts on both clients, the calendar gains the whole retry layer and with it
defect 3, and both gain the `Dropped` kind, which is what makes the first request after a laptop
wakes succeed instead of showing an error.

## Credentials, the build script, and the reqwest split

There is one Google Cloud project, `margin-500217`, one OAuth desktop client, and three
byte-identical copies of `google-credentials.json` in three repo roots, all gitignored, with only the
example files committed. See [guidelines/distribution.md](guidelines/distribution.md).

The mechanism the calendar and Mail share is right and stays: `src-tauri/build.rs` copies the file
from the repo root into `OUT_DIR`, falling back to `google-credentials.example.json` when the real
file is absent, and the auth code pulls it in with
`include_str!(concat!(env!("OUT_DIR"), "/google-credentials.json"))` (mail auth.rs:63). Nothing is
read at runtime, and a clone with no credentials compiles and fails at the first connect with the
"not set up yet" sentence. The two `build.rs` files are byte identical, so this becomes
`margin_google::build::embed_credentials()` called from each app's `build.rs`.

margin is the app to fix. Delete the runtime read at gdrive.rs:41-42 outright, point
`CREDENTIALS_JSON` (gdrive.rs:22-23) at `OUT_DIR`, and add the build script call. That takes the
build machine's absolute path out of the binary, removes the case where a developer's on-disk file
outranks the compiled one, and makes a fresh clone compile. margin's example file carries only an
`installed` block where the other two carry `android` and `ios`; bring it to the same shape.

**reqwest** settles at 0.13, with two corrections. The three hand-written helpers in Mail are
avoidable and should go: `form_body` (auth.rs:502-511), `query_string` (api.rs:723-729) and
`url_with` (api.rs:732-738) exist because `form` is a feature in reqwest 0.13 and Mail sets
`default-features = false` without listing it (reqwest 0.13.1 `Cargo.toml`:
`form = ["dep:serde", "dep:serde_urlencoded"]`, absent from `default`). In 0.12 `serde_urlencoded`
was an unconditional dependency, which is why the calendar never noticed; adding `"form"` deletes all
three and the comments explaining them, and should happen before the shared crate copies the
workaround forward. Second, Mail already compiles two reqwest majors: `Cargo.lock` has 0.12.28 and
0.13.1, the older pulled in by `css-inline 0.21.2`, so one version in the tree is not reachable
through these crates and one version in code we write is. Keep Mail's feature set as the crate's,
plus `form`: `rustls`, `webpki-roots`, `json`, `gzip`, `http2`.

## Sync engines: do not share one

Both are honestly described as "incremental sync of a Google resource into a local SQLite mirror with
a sync token, a poll loop and an event stream to the UI". That sentence is true of both and it is
where the similarity ends. Six things differ in kind, not in degree.

| | Margin Calendar | Margin Mail |
|---|---|---|
| Cursor | one sync token per calendar in a column, plus a `calendarList` token in `meta` keyed by account (store/schema.rs:43, pull.rs:26, 204) | one per account database, Gmail's `historyId` (mirror/write.rs:29) |
| Commit point | end of the page chain, because `nextSyncToken` only arrives on the last page; the whole chain is one transaction and an interruption restarts it (pull.rs:136-195) | as soon as changes are on disk and deliberately before hydration, so a failed crawl does not re-read the log (changes.rs:51-56) |
| Cursor expiry | 410 drops that calendar's rows and cursor and re-syncs it alone (pull.rs:105-132) | drops nothing; `reconcile` lists the window into a TEMP TABLE and diffs locally, because the mirror holds decisions the state database joins against (changes.rs:141-228) |
| Fetch | one phase, `events.list` returns whole events | ids, then metadata in batches of 50, then bodies, with a `hydrated` column so an interrupted crawl resumes across restarts (hydrate.rs:1-9, changes.rs:69-92) |
| Writes | `If-Match`, and a 412 is a lost race that is surfaced and never retried with the etag dropped, since dropping it is the clobber the check prevents (api.rs:310-360, push.rs:5-8) | no etag exists, so writes are declarative: what the labels should be, not what to do to them (api.rs:649-650), which is why they are safe to replay |
| The seam | `Transport`, six methods, every one naming a Google Calendar type, one implementation and a test stub (transport.rs:15-62) | `Provider`, fourteen methods, no Google type at all, three implementations: Gmail, IMAP and a 612-line fake (provider/mod.rs:166-249) |

An abstraction over "a token per collection, committed at the end of a chain, with etags" and "a log
per account, committed halfway, with declarative writes and a quota accountant in the middle" would
be larger and harder to read than either engine it replaced. Do not build it.

What is worth extracting is the scaffolding, the same to the line in places: roughly 150 to 250 lines
into `margin-db` and a small `margin-sync`. The poll loop shape, spawn then `FIRST_PASS_SECS = 2`
then an interval chosen by window focus, with `fn focused(app)` byte-identical (calendar
sync/mod.rs:39, 205-223; mail sync/mod.rs:51, 374-393), the intervals staying per app at 60/300 s
against 12/60 s. The `Sink` trait, so a pass runs in a test with a recorder instead of an `AppHandle`
(calendar sync/mod.rs:69-78, mail sync/mod.rs:238-249). The connection-borrowing seam, carrying the
same justification verbatim in both, that nothing inside may await because the guard is a std one and
holding it across a suspension point would make the future non-Send (calendar sync/mod.rs:114-115,
mail sync/mod.rs:220-221). The push-before-pull budgets, 20 s for the outbox in a pass and 4 s at
quit (calendar sync/mod.rs:42, 44; mail outbox.rs:30, 32). And the `meta` table with its
`schema_version` key, the refusal to open a database written by a newer build and forward-only
numbered steps inside a transaction (calendar store/schema.rs:100-136, mail mirror/schema.rs:21-63),
which belongs in `margin-db`. The four event names both apps agree on, `store-changed`,
`sync-progress`, `auth` and `menu-action`, are fixed in `margin-ipc` rather than in the engines. All
of this buys consistency rather than deletion, which is the honest reason to do it.

## Backup

**margin's** is whole-file mirroring. `collect_local_files` gathers `*.margin` books and the custom
dictionary (gdrive.rs:576-600), hashes each, and uploads anything whose hash moved
(gdrive.rs:810-835). `gdrive_sync` downloads any remote file with no local counterpart and skips any
that has one (gdrive.rs:877-884), so the local copy always wins and there is no merge. Nothing is
encrypted: the books go up as they are, into a visible folder called `margin` at the Drive root under
`drive.file`, which `docs/publishing.md:131, 162-164` defends as deliberate.

**Margin Mail's** is an append-only encrypted journal. Segments of 500 records (backup/mod.rs:48)
named `<account-hash>/<device-id>/<first>-<last>.seg`, the account hash a 128-bit truncated SHA-256
of the address so a folder listing is not a list of somebody's email addresses (mod.rs:52-77). Each
segment is sealed with XChaCha20-Poly1305 with its own name as additional authenticated data, so a
store that reorders, replays or moves a segment gets a decryption failure rather than a wrong answer
(crypto.rs:60-105). Merge is a union, because a device only ever writes under its own sequence. The
key comes from a 24-word BIP39 phrase through Argon2id at RFC 9106's second profile, 64 MiB, three
passes, one lane (phrase.rs:21-48), with a constant salt because a second device has the phrase and
nothing else. The store is a three-method trait, `put` / `get` / `list` (store.rs:20-30), the
intersection of Drive's REST API and S3, with two implementations: Drive and S3 sigv4 signed by hand.

**The one genuine overlap is five HTTP functions.** margin's `ensure_folder`, `find_file`,
`list_in_folder`, `upload_file` and `download_file` (gdrive.rs:361-496) and Mail's `ensure_folder`,
`find_file`, `list_folder`, `upload` and `download` (google/drive.rs:100-256) are the same calls
written twice. Mail's is better in five specific ways: it pages at 1000 rather than 100 (drive.rs:176
against gdrive.rs:432), it escapes the Drive query language (drive.rs:47-49), it randomises the
multipart boundary where margin hardcodes `margin7f3e2a1b9c8d` (drive.rs:74-78 against
gdrive.rs:455), it percent-encodes the file id into the path (drive.rs:228-231), and it returns a
classified `ApiError`. Move Mail's five into `margin-google` as a `drive` module with the
`drive.file` scope constant, `escape`, and the `FOLDER_NAME = "margin"` root, which both apps hold as
their own string literal today (gdrive.rs:16, drive.rs:28); each app then names its own subfolder,
`margin/mail/` for Mail.

**Should the newer design replace the older? No, and not because of effort.** They back up different
things. Mail's journal is append-only records of decisions, which is what makes a union merge
correct; margin's payload is a book file a person edits on two machines, where a union is meaningless
and the answer is last-writer-wins or a real merge. Wrapping books in an encrypted journal would also
break the property `docs/publishing.md` defends, that the folder in a person's Drive is legible and
their books are files they can open. What margin should take is narrower: the five Drive verbs,
`ApiError`, and the sealed store for its refresh token. The whole-file mirror stays, and its real
defect, no merge and local always wins, is a product decision for margin's own docs rather than
something this consolidation should quietly change.

## Per app, in order

**Margin Mail** first, because it is the source for all three crates.

1. Define `listen_for_redirects` in `src-tauri/src/lib.rs`, ported from calendar lib.rs:150-172.
   Defect 2, and it blocks any mobile build.
2. Add `"form"` to the reqwest features (Cargo.toml:37-43); delete `form_body`, `query_string`,
   `url_with`.
3. Extract `google/secrets.rs` into `margin-secrets` as a `Store` value, on-disk format byte for
   byte. Rewire `google/auth.rs`, `backup/crypto.rs:119-138` and `backup/r2.rs:29`.
4. Extract the error, retry, backoff and `Quota` half of `google/api.rs` into `margin-http`, with
   `Quota::new(6_000, 60_000)` at the call site and the `Call` unit table staying in Mail.
5. Extract `google/auth.rs`, `google/browser.rs`, `build.rs` and the five verbs of `google/drive.rs`
   into `margin-google`. Mail implements `AccountSink` over `crate::accounts`. `auth::remove`'s
   database teardown and its `keep_data` flag stay in Mail; the crate's `disconnect` does the revoke,
   the token and the session and nothing else.

**Margin Calendar** second, and mostly deletion.

1. Upgrade `chacha20poly1305` to 0.11 (Cargo.toml:40) and adopt `margin-secrets`. No data migration.
   Keep `reference()` locally: two lines serving a column no other app has.
2. Adopt `margin-http`. Replace `error_for` (api.rs:191-197) with the crate's, keeping the 410 and
   412 mappings on top, and wrap `push::drain` in `with_retry_no_replay`, because an event write
   carrying an etag must not be replayed. Defect 3.
3. Adopt `margin-google`. Delete `google/auth.rs`, `google/browser.rs` and `build.rs`; implement
   `AccountSink` over `store::write::upsert_account`; pass the existing scope string as a slice for
   `base_scopes` and an empty `required_scopes`; move `listen_for_redirects` out of lib.rs.
4. Take the two extra `auth` event fields as empty arrays and update `src/` to ignore them.

**Margin (the writing studio)** last, and it gains the most.

1. Before anything else, stop writing the refresh token in plaintext. Add `margin-secrets`, move
   `BackupState.refresh_token` (gdrive.rs:68) into the sealed store, and have `load_state` migrate an
   existing plaintext token on first read and then clear the field. Defect 1, worth doing on its own
   schedule if the rest slips.
2. Fix the credentials path: add the build script call, point `CREDENTIALS_JSON` (gdrive.rs:22-23) at
   `OUT_DIR`, delete the runtime read (gdrive.rs:41-42), bring the example file to the three-block
   shape. Defect 5.
3. Adopt `margin-http` for both clients (gdrive.rs:25, updates.rs:6). Defect 4.
4. Adopt `margin-google`, deleting roughly 330 lines of gdrive.rs: the PKCE, the inline listener, the
   token endpoint calls, the session. `base_scopes` is `["openid", "email", "drive.file"]` and
   `required_scopes` is `["drive.file"]`, which margin already enforces by hand (gdrive.rs:181-186,
   522-532) and the calendar still does not.
5. Adopt the crate's Drive verbs, deleting gdrive.rs:361-496.
6. Make `gdrive_disconnect` (gdrive.rs:777-803) report the revoke outcome, and add the settings
   sentence saying it signs the person out of every Margin app. Defect 6. What is left of `gdrive.rs`
   afterwards is the file mirror, the hash ledger and the Tauri commands.

## What stays per app

The sync engines, whole, and the provider and transport traits. Every scope list, poll interval and
quota budget, because they are facts about a resource rather than about OAuth. The per-call unit
table in Mail's `google/api.rs:119-176`, which names Gmail methods. The account registry itself, a
SQLite row in one app and `accounts.json` in the other, behind `AccountSink`. Mail's `auth::remove`
database teardown and its `keep_data` flag. Mail's backup journal, phrase derivation and R2 store,
and margin's whole-file Drive mirror. `imap/discover.rs`'s Chrome user agent, correct there and wrong
everywhere else. And all the app copy: the consent explanation, the disconnect warning, the
not-set-up sentence, because copy is product and the crate only supplies the app name it is built
from.
