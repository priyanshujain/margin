# Google, OAuth, secrets, HTTP, sync and backup across the four apps

Read directly off disk on 2026-09-06. Every claim below has a file and a line behind it. No secret
values are reproduced anywhere in this file.

Margin Docs (`rust/margin-editor`) is out of scope on the evidence: its `src-tauri/Cargo.toml` has
no `reqwest`, no `chacha20poly1305` and no Google anything. It is the control case, and the only
thing it shares with the other three is `library.rs`-shaped filesystem helpers.

## The size of the thing

| Package | Files | Lines |
|---|---|---|
| `margin-caledar/src-tauri/src/google/` | 5 | 2,258 |
| `margin-mail/src-tauri/src/google/` | 10 | 6,077 |
| `margin/src-tauri/src/gdrive.rs` | 1 | 953 |
| `margin-mail/src-tauri/src/backup/` | 7 | 1,914 (1,264 without tests) |
| `margin-caledar` sync + store | 9 | 3,784 (2,982 without tests) |
| `margin-mail` sync + mirror + provider | 16 | 10,215 (7,210 without tests) |

## The two OAuth implementations are one implementation

Margin Mail's `google/auth.rs` opens by saying so: "ported from Margin Calendar's `google/auth.rs`,
which took it from margin's `gdrive.rs`" (margin-mail auth.rs:1-4). The calendar's opens the same
way about margin (margin-caledar auth.rs:1-7). This is not a family resemblance, it is a copy with
edits, and it is measurable.

Ignoring the `#[cfg(test)]` modules at the foot of each file:

| File | Calendar lines | Mail lines | Calendar lines present verbatim in Mail |
|---|---|---|---|
| `google/auth.rs` | 899 | 1,146 | 810 (90%) |
| `google/browser.rs` | 420 | 419 | 416 (99%) |
| `google/secrets.rs` | 356 | 351 | 326 (92%) |
| `src-tauri/build.rs` | 29 | 29 | 29 (100%) |

1,552 of the calendar's 1,675 non-test lines exist unchanged in Margin Mail. `browser.rs`, which is
the whole iOS `SFSafariViewController` and Android Custom Tab consent surface, differs in seven
lines and all seven are comment prose about which app the sheet appears over.

Everything below is identical in both, line for line:

- PKCE: `random_b64`, `pkce_challenge` and the RFC 7636 test vector (calendar auth.rs:228-238 and
  904-909, mail auth.rs:252-262 and 1170-1175).
- The loopback listener: `write_http_message`, `Redirect`, `request_path`, `parse_redirect`,
  `await_code`, the `CANCELLED` constant and the `access_denied` special case (calendar
  auth.rs:244-374, mail auth.rs:307-437). Same 8 KiB buffer, same 150 ms poll, same 5 s read
  timeout, same favicon skip.
- Credentials loading: `CredentialsFile` with `installed` / `android` / `ios`, the `platform_client`
  flag, the `YOUR_CLIENT_ID` sentinel check (calendar auth.rs:68-157, mail auth.rs:88-182).
- Token endpoint: `TokenResponse`, `with_secret`, `exchange_code`, `refresh_access_token`,
  `fetch_email` (calendar auth.rs:376-477, mail auth.rs:439-572).
- `id_token` handling: parsed, never signature-verified, with the same justification that TLS
  already proved it (calendar auth.rs:402-408, mail auth.rs:472-478).
- Expiry and skew: `EXPIRY_SKEW_SECS = 60` and
  `now() + expires_in.saturating_sub(EXPIRY_SKEW_SECS)` (calendar auth.rs:48 and 875, mail
  auth.rs:61 and 1122). No clock-skew handling beyond that constant, in either.
- `valid_access_token`, single-flight by holding the tokio mutex across the refresh await, with the
  same paragraph explaining that this also serialises refreshes across accounts (calendar
  auth.rs:846-877, mail auth.rs:1093-1124).
- The deep-link path: `Pending`, `handle_redirect` taking rather than reading the verifier,
  `abandon_pending`, `connect_by_deep_link`, `callback_scheme`, the 900 s mobile timeouts (calendar
  auth.rs:204-210, 633-747; mail auth.rs:228-234, 807-923).
- Multi-account model: `AuthState { sessions: Mutex<HashMap<String, Session>> }`, keyed on the
  Google `sub` with the email as fallback (calendar auth.rs:214-219 and 799, mail auth.rs:238-243
  and 975).
- The build script that embeds `google-credentials.json` from `OUT_DIR`, falling back to the example
  file so a fresh clone compiles.

### What actually differs

Six things, and they are the whole design space a shared crate has to leave open.

1. **Scopes.** The calendar has one constant string (`SCOPES`, auth.rs:28). Mail has a six-entry
   `BASE_SCOPES` array, a `REQUIRED_SCOPE` an account is refused for, `scope_string` for adding
   extras, and `granted_scopes`/`missing_required` reading what Google actually granted back off the
   token response (mail auth.rs:29-41, 274-305, 980-994). The calendar never reads the `scope` field
   at all; its `TokenResponse` does not have one (calendar auth.rs:376-385).
2. **Re-consent.** Mail has `grant()` (auth.rs:725-733) because installed apps get no incremental
   authorization, so picking up `calendar.events` to answer an invite means the whole consent again.
   The calendar never needs a second scope.
3. **`login_hint` and `prompt`.** Mail's `auth_url` takes an optional hint and switches `prompt`
   between `consent` and `select_account consent` accordingly (auth.rs:607-633). The calendar always
   sends `select_account consent` (auth.rs:519-530).
4. **Revocation.** The calendar's `revoke` discards the result (auth.rs:479-485). Mail's keeps it
   and reports "the account was removed from this device, but Google could not be reached to revoke"
   (auth.rs:584-598, 1086-1090). Mail also documents that one OAuth client covers the suite, so a
   revoke signs the person out of every Margin app (auth.rs:1026-1029, google/mod.rs:53-56).
5. **Where the account record lands.** The calendar writes a row through
   `store::write::upsert_account` inside a SQLite store (auth.rs:806-808). Mail writes
   `accounts.json` through `crate::accounts::upsert` (auth.rs:997-1003), because every account owns
   its own database and the account list has to be readable before any of them is open
   (accounts.rs:4-8).
6. **HTTP body building.** Mail is on reqwest 0.13, where `.form()` sits behind a feature this build
   does not enable, so it hand-rolls `form_body` over `url::form_urlencoded` (auth.rs:502-511). The
   calendar uses `.form()` (auth.rs:438-444). This is the only place the version split shows up in
   the auth code.

Point 6 is worth naming as the pattern: the reqwest 0.12/0.13 gap has already cost Margin Mail three
hand-written helpers that the other two get from the library (`form_body` auth.rs:505-511,
`query_string` api.rs:723-729, `url_with` api.rs:732-738).

## Secret storage

`google/secrets.rs` is the same file twice. The whole diff is: the `SERVICE` and `KEY_CONTEXT`
constants (calendar secrets.rs:31 and 34, mail secrets.rs:42 and 46), a `reference()` helper the
calendar needs for its `accounts.keychain_ref` column and mail dropped, a `tempfile` in one test,
and the chacha20poly1305 0.10 to 0.11 API change.

The scheme itself, identical in both: an XChaCha20-Poly1305 blob at `tokens.enc` in the app data
directory, one base64 entry per account id in a `BTreeMap`, a 32-byte per-install random salt at
`tokens.salt`, and the key derived as `SHA256(KEY_CONTEXT || salt || machine_id)`
(mail secrets.rs:101-119). `machine_id` is `/etc/machine-id` on Linux, `IOPlatformUUID` scraped out
of `/usr/sbin/ioreg` on macOS, and deliberately empty on iOS and Android because the sandbox is the
real boundary there and a reinstall would rotate the identifier (mail secrets.rs:128-189). Files are
written 0600 from creation rather than chmodded afterwards (mail secrets.rs:236-239 doc comment).

The "why not keyring" paragraph is near-identical in both `Cargo.toml` files (margin-caledar
Cargo.toml:36-40, margin-mail Cargo.toml:61-64) and the long version is in the file headers
(calendar secrets.rs:1-27, mail secrets.rs:1-28): macOS ties a keychain item's ACL to the code
signature so every ad-hoc rebuild re-prompts, and `keyring` has no Android backend at all.

**Version drift.** `chacha20poly1305 = "0.10"` in the calendar, `"0.11"` in mail. Mail carries the
migration note (secrets.rs:208-211): 0.11 moved to `hybrid-array`, `Key::from_slice` and
`XNonce::from_slice` are deprecated, and the array conversions now carry the length in the type, so
the one panic those calls had is a compile error. The on-disk format is unchanged, so this is purely
an API-surface difference and any shared crate should be on 0.11.

Mail reuses this store for two things that are not OAuth tokens: the backup key under the id
`"margin-mail backup key"` (backup/crypto.rs:41, 119-138) and the R2 credentials under
`"margin-mail r2 credentials"` (backup/r2.rs:29). Both comments note that neither id can collide
with an account id, because a Google `sub` is digits and an address cannot carry a space. That is a
good sign for extraction: the module already works as a general sealed-key-value store and only its
two constants are app-specific.

## Credentials and the Google Cloud project

The mechanism, in both mail and the calendar: `src-tauri/build.rs` copies
`google-credentials.json` from the repo root into `OUT_DIR`, falling back to
`google-credentials.example.json` when the real file is absent, and `auth.rs` pulls it in with
`include_str!(concat!(env!("OUT_DIR"), "/google-credentials.json"))` (mail auth.rs:63, calendar
auth.rs:50). So the client id and the desktop client secret are compiled into the binary. Nothing is
read at runtime. A clone with no credentials compiles and fails at the first connect with the
"not set up yet" sentence rather than failing to build (build.rs:8-10).

Margin does it differently and worse. `gdrive.rs:41-42` builds a path from
`CARGO_MANIFEST_DIR/../google-credentials.json` and tries to `fs::read_to_string` it **at runtime**,
falling back to the `include_str!` copy only when that read fails. On a shipped app the path does not
exist so the fallback always wins, but the build machine's absolute path is baked into the binary
and a developer's on-disk file silently takes precedence over what was compiled.

The three `google-credentials.json` files in `margin`, `margin-caledar` and `margin-mail` are
byte-identical (same SHA-1). All three are `.gitignore`d and untracked; only the example files are
committed. Margin additionally keeps the raw console download at its repo root, also ignored via a
`/client_secret_*.json` rule (margin/.gitignore:31).

So there is **one** Google Cloud project and **one** OAuth desktop client for the whole suite,
copied into three repos by hand. Margin Mail's code knows this and says so on the revoke path
(auth.rs:1026-1029). Margin's does not: `gdrive_disconnect` revokes the grant without telling anyone
it has just signed them out of the calendar and the mail client too (gdrive.rs:776-803). The example
files also differ: mail and the calendar carry `android` and `ios` blocks, margin's carries only
`installed`.

## margin's gdrive.rs, the third client

It authenticates with the same flow, written independently and earlier. Same PKCE
(gdrive.rs:159-169), same loopback listener (gdrive.rs:199-275, but inline in one 60-line function
rather than the calendar's four testable pieces), same `exchange_code` / `refresh_access_token` /
`fetch_email` (gdrive.rs:318-359), same `access_type=offline` consent URL (gdrive.rs:751-759). About
330 of its 953 lines are the OAuth flow.

What it does differently, in every case for the worse:

- **The refresh token is stored in plaintext.** `BackupState.refresh_token` is a plain field
  (gdrive.rs:71) serialised into `backup.json` with `serde_json::to_string_pretty` and written by
  `crate::project::atomic_write` (gdrive.rs:153-157), which sets no file mode (project.rs:13-29).
  The other two apps seal the same kind of token for the same grant. The weakest store in the suite
  sets the real security level, so this is the one finding here I would act on before any
  refactoring.
- **No timeouts on the HTTP client at all**: `LazyLock::new(reqwest::Client::new)` (gdrive.rs:25).
  The calendar's file header calls this out by name as fix number one (calendar auth.rs:1-4) and
  nobody went back and applied it.
- **No single-flight refresh**: `valid_access_token` reads the session, drops the lock, then awaits
  the refresh (gdrive.rs:498-521), so N concurrent callers each fire their own. Fix number two in the
  calendar's header, also never backported.
- **No CSRF constant-time discipline and no `Redirect` enum**, so the state check and the error
  paths are inline and untested. There are no tests in `gdrive.rs` at all; the calendar has nine and
  mail has eighteen for the same code.
- Its one advance on the calendar is that it reads the granted `scope` back and refuses a token
  without `drive.file` (gdrive.rs:181-186, 522-532, 704-712). That idea survived into Margin Mail
  as `granted_scopes` / `missing_required` and never reached the calendar.

## HTTP clients

Seven `reqwest::Client` constructions across the three apps, all different, none sharing a builder.

| Where | connect | total | Other |
|---|---|---|---|
| margin gdrive.rs:25 | none | none | `Client::new()` |
| margin updates.rs:6 | none | none | `Client::new()` |
| calendar auth.rs:52-58 | 10 s | 30 s | nothing |
| mail auth.rs:65-78 | 10 s | 30 s | h2 keepalive 20/10, tcp keepalive 30 s |
| mail api.rs:86-108 | 10 s | 60 s | pool idle 30 s, h2 keepalive, tcp keepalive |
| mail backup/r2.rs:40-45 | 10 s | 120 s | nothing |
| mail attachments.rs:67-78 | 5 s | 10 s | `referer(false)`, redirects limited to 3 |
| mail unsubscribe.rs:81-99 | 5 s | 20 s | `referer(false)`, same-host-only redirect policy |
| mail imap/discover.rs:543-554 | 15 s | none | spoofed Chrome user agent, 3 redirects |

Versions and features:

- margin: `reqwest 0.12`, `default-features = false`, `rustls-tls`, `json` (Cargo.toml:39).
- calendar: identical (Cargo.toml:29).
- mail: `reqwest 0.13`, `default-features = false`, `rustls`, `webpki-roots`, `json`, `gzip`,
  `http2` (Cargo.toml:37-43), justified as "gzip because Gmail's JSON compresses by an order of
  magnitude and hydration is thousands of responses; http2 because a batch of 50 and the poll loop
  share one connection".

**User agent**: nothing sets one except `imap/discover.rs`, and that one is a Chrome spoof, which is
deliberate for autodiscovery and wrong for anything else. All Google traffic from all three apps
goes out as reqwest's default UA.

**Retry, backoff, rate limits.** Only Margin Mail has any. `google/api.rs` carries the whole of it:

- `ApiError` with seven variants, including `Dropped` for a connection that died mid-request as
  distinct from `Offline` (api.rs:183-203, 233-246).
- `error_for`, classifying 401 / 403-by-reason / 404 / 429 / 408 / 5xx, reading Google's reason out
  of `error.status`, `error.errors[].reason` and `error.details[].reason` because a missing scope
  only appears in the third (api.rs:341-440).
- `strip_urls`, which drops whole sentences carrying a link out of an error message on the grounds
  that a person reading a toast cannot follow one (api.rs:276-305).
- `retry_after` (api.rs:443-452), truncated exponential `backoff_ms` with jitter capped at 64 s
  (api.rs:612-636), `with_retry` and `with_retry_no_replay` for calls that must not be made twice
  (api.rs:651-696), and a two-step fast retry for dropped connections (api.rs:642).
- `Quota`: a per-account rolling 60-second ledger against Gmail's 6,000 units, with a per-call unit
  table (api.rs:119-176, 496-607).

The calendar has **none** of this. Its `error_for` maps 410 and 412 and sends everything else to
`ApiError::Other` (margin-caledar api.rs:191-197). Grepping the whole calendar crate for `429`,
`Retry-After`, `backoff` or `sleep` returns exactly one hit, and it is the 150 ms poll inside the
loopback listener. A 429 from Google Calendar therefore reaches `push::drain`, is counted as a real
attempt, and five of them retire the write permanently (calendar push.rs:29, 344-345, 375-381). That
is a live bug, not a stylistic gap.

## Sync engines: how much is really shared

Both are honestly described as "incremental sync of a Google resource into a local SQLite mirror
with a sync token, a poll loop and an event stream to the UI". That description is true of both and
it is also where the similarity ends. I read both in full and I do not think a shared sync engine is
the right conclusion.

### What genuinely is the same

The scaffolding, and it is the same to the line in places.

- The poll loop. Spawn a task, first pass at `FIRST_PASS_SECS = 2`, then an interval chosen by
  window focus (calendar sync.rs:39, 205-217; mail sync/mod.rs:51, 374-387). `fn focused(app)` is
  byte-identical in both (calendar sync.rs:219-223, mail sync/mod.rs:389-393). Intervals differ
  because the resources do: 60/300 s for the calendar, 12/60 s for mail, each with the unit cost
  worked out in a comment.
- The `Sink` trait, so a pass can run in a test with a recorder behind it instead of an `AppHandle`
  (calendar sync.rs:69-78, mail sync/mod.rs:238-249). Both `AppSink`s emit `sync-progress` and call
  `emit_store_changed`.
- The connection-borrowing seam, with the same justification sentence in both files verbatim:
  "Nothing inside may await: the guard is a std one, so holding it across a suspension point would
  make the future non-Send" (calendar sync.rs:114-115, mail sync/mod.rs:220-221). The calendar has it
  as a free function over `Store`; mail has it as a `Store` trait with a `Scoped` impl.
- Push before pull, with the same reason ("a write that has just landed comes back as the server's
  own row in the same pass rather than a tick later"): calendar sync.rs:135-136, mail engine.rs:3-4.
- The budgets: 20 s for the outbox in a pass and 4 s at quit, in both (calendar sync.rs:42 and 44,
  mail outbox.rs:30 and 32), with the same doc comment about racing the frontend's close-request
  timeout.
- Schema migration: a `meta` key/value table, a `schema_version` key, a refusal to open a database
  written by a newer build, forward-only numbered steps inside a transaction (calendar
  store/schema.rs:100-136, mail mirror/schema.rs:21-63). About 40 lines each, near-identical.
- The webview event surface. Both apps emit exactly four events and they have the same names:
  `store-changed`, `sync-progress`, `auth`, `menu-action`. `AuthEvent` is the same struct with two
  extra fields in mail (calendar dto.rs:172-181, mail dto.rs:896-910). `SyncStatus` shares five of
  its fields (calendar dto.rs:147-155, mail dto.rs:861-877).

### What is not the same, and cannot be

- **The cursor model differs in kind.** The calendar keeps one sync token per calendar in a column
  (store/schema.rs:43) plus one `calendarList` token in `meta` keyed by account (pull.rs:26, 204).
  Mail keeps one cursor per account database, which is Gmail's `historyId` (mirror/write.rs:29).
- **Commit points are opposite.** The calendar cannot write anything until the page chain is
  exhausted, because `nextSyncToken` only arrives on the final page, and it says so at length
  (pull.rs:1-7, 136-171, 173-195: one transaction for the whole chain). An interrupted chain
  restarts from the beginning. Mail commits the cursor as soon as the changes are on disk and
  *before* hydration, deliberately (changes.rs:51-56), so a crawl that fails afterwards does not
  re-read the change log.
- **Recovery is opposite.** A 410 in the calendar drops that one calendar's rows and cursor and
  re-syncs it alone (pull.rs:105-132). An expired history log in mail drops nothing: `reconcile`
  lists the window into a TEMP TABLE and diffs it locally (changes.rs:141-228), because the mirror
  holds decisions the state database is joined against and dropping it would be destructive.
- **Mail has a two-phase fetch and the calendar has no use for one.** Ids, then metadata in batches
  of 50, then bodies, with an `hydrated` column so an interrupted crawl resumes across restarts
  (hydrate.rs:1-9, 22, changes.rs:69-92). `events.list` returns whole events, so this entire axis is
  absent from the calendar.
- **Conflict resolution is opposite.** The calendar sends `If-Match` and treats a 412 as a lost race
  that is surfaced and never retried with the etag dropped, since dropping it is the clobber the
  check exists to prevent (api.rs:310-332, 337-360; push.rs:5-8). Mail has no etag because Gmail has
  none. Its writes are declarative ("say what the labels should be rather than what to do to them",
  api.rs:649-650), which is exactly why they are safe to replay and the calendar's are not.
- **Coalescing.** Mail folds consecutive writes to the same message set into one outbox row, per
  field for flags and per label for labels (outbox.rs:63-190). The calendar does not, and its outbox
  row carries `calendar_id`, `event_id`, `original_start`, `scope` and `etag` (store/schema.rs:75-87)
  where mail's carries `op`, `payload`, `thread_key` and `hold_until` (mirror.sql:120-129).
- **Failure policy.** Mail has a circuit breaker at four consecutive failures with a five-minute
  cooldown, a quiet-first-failure rule, and per-error-kind chip wording (engine.rs:22-33, 190-228),
  plus a body-cache poison set (engine.rs:65, 250-281). The calendar reports every failure.
- **The provider seam is not the same seam.** Mail's `Provider` has fourteen methods and names no
  Google type at all (provider/mod.rs:166-249). It has three implementations: Gmail
  (gmail.rs:378), IMAP (imap/provider.rs:1439) and a 612-line fake. The calendar's `Transport` has
  six methods and every one of them names a Google Calendar type in its signature
  (`CalendarListPage`, `RawEvent`); it has one real implementation and a test stub
  (transport.rs:15-62, tests.rs:60). So mail's is a provider abstraction and the calendar's is a
  test seam. They look alike (both `impl Future + Send` on a `Sync` trait, no `async_trait`, and the
  mail file says so: provider/mod.rs:13-14) and they are doing different jobs.

### Verdict

A shared OAuth crate is obviously right and the numbers support it without argument. A shared sync
engine is not. What the two engines agree on is the shape around the work: how a pass is scheduled,
how a store connection is borrowed, how progress reaches the webview, and what an outbox row's
lifecycle looks like. That is worth perhaps 150 to 250 lines of traits and small helpers, and
extracting it would buy consistency rather than deletion. The interiors have no overlap worth
having: one syncs whole objects with etags against a per-collection token that can only be committed
at the end of a chain, the other syncs ids then metadata then bodies against a per-account log with a
quota accountant in the middle and a checkpoint halfway through.

The more useful extraction on this side is not the engine at all. It is Margin Mail's
`google/api.rs` error and retry layer: `ApiError`, `error_for`, `reasons`, `explain`, `strip_urls`,
`retry_after`, `backoff`, `with_retry` and `Quota`. That is roughly 350 lines the calendar visibly
needs and does not have, and moving it would fix the 429 bug above rather than merely deduplicating
something.

## Backup

Two designs with almost nothing in common except the Drive verbs underneath.

**margin's** is whole-file mirroring. `collect_local_files` gathers `*.margin` books and the custom
dictionary (gdrive.rs:576-600), hashes each one, and uploads anything whose hash moved
(gdrive.rs:810-835). `gdrive_sync` downloads any remote file that has no local counterpart and skips
any that does, so the local copy always wins and there is no merge at all (gdrive.rs:877-884).
Everything goes into one visible folder called `margin` at the root of the user's Drive under
`drive.file`, which the publishing doc defends as deliberate (docs/publishing.md:131, 162-164).
Nothing is encrypted; the books go up as they are.

**Margin Mail's** is an append-only encrypted journal. Segments are 500 records each
(backup/mod.rs:48), named `<account-hash>/<device-id>/<first>-<last>.seg` (mod.rs:76-78), where the
account hash is a truncated SHA-256 of the address so a folder listing is not a list of somebody's
email addresses (mod.rs:52-67). Each segment is sealed with XChaCha20-Poly1305 with its own name as
additional authenticated data, so a store that reorders, replays or moves a segment gets a
decryption failure rather than a wrong answer (backup/crypto.rs:60-105). Merge is a union rather than
a conflict resolution, because a device only ever writes under its own sequence (mod.rs:179-181),
and a pass reads before it writes so a mistyped phrase fails before it has added anything
(mod.rs:131-133).

The key comes from a 24-word BIP39 phrase through Argon2id at RFC 9106's second recommended profile,
64 MiB, three passes, one lane (backup/phrase.rs:21-48). The salt is a constant and the file explains
why: a second device has the phrase and nothing else, so every input has to be reachable from the
phrase alone, and 256 bits from the wordlist is what is doing the work (phrase.rs:42-48). The phrase
is shown once and never again, which `backup_phrase` enforces by refusing the second call
(mod.rs:364-373). The derived key is sealed through `google::secrets`, deliberately reusing that
module rather than reimplementing it (crypto.rs:107-117).

The store is a three-method trait, `put` / `get` / `list` (backup/store.rs:20-30), chosen as the
intersection of Drive's REST API and S3 so that a second implementation is an afternoon. There are
two: Drive (backup/drive.rs, 172 lines) and S3 sigv4 signed by hand for R2 (backup/r2.rs, 426 lines,
with the reasoning against `aws-sdk-s3` at r2.rs:10-13).

**What the two share** is five HTTP functions. margin's `ensure_folder`, `find_file`,
`list_in_folder`, `upload_file` and `download_file` (gdrive.rs:361-496) and Margin Mail's
`ensure_folder`, `find_file`, `list_folder`, `upload` and `download` (google/drive.rs:100-256) are
the same calls written twice. Mail's is better in five specific ways: it pages at 1000 rather than
100 (drive.rs:176 against gdrive.rs:432), it escapes the Drive query language properly
(drive.rs:47-49), it randomises the multipart boundary where margin hardcodes one string
(drive.rs:74-78 against gdrive.rs:455), it percent-encodes the file id into the path
(drive.rs:228-231), and it returns a classified `ApiError`.

Both write into `margin` at the Drive root under `drive.file`, and both hold that as a constant in
their own file (gdrive.rs:16, google/drive.rs:28). Mail nests itself at `margin/mail/` and its file
header notes the architecture document had this wrong until recently (google/drive.rs:1-16). So the
suite already has a folder convention that exists as two unrelated string literals.

margin's docs do not contain a Drive backup spec. `docs/` has one file, `publishing.md`, and its
only Drive content is the App Store entitlement justification at lines 131 and 160-170. The README
is 13 lines and does not mention Drive.

## Defects and drift found on the way

1. **Margin Mail will not compile for mobile.** `lib.rs:307` calls `listen_for_redirects(handle)`
   under `#[cfg(mobile)]` and that function is not defined anywhere in the crate. The calendar has
   it (margin-caledar lib.rs:150-172). The call site was copied and the definition was not. Desktop
   builds are unaffected, which is why it has not been noticed.
2. **margin keeps a Google refresh token in plaintext** in `backup.json` (gdrive.rs:71, 153-157).
   Same OAuth client and same grant as the two apps that seal theirs.
3. **The calendar has no rate limit handling**, so a 429 is counted as a failed attempt and five of
   them permanently retire a queued write (api.rs:191-197 with push.rs:29, 375-381).
4. **margin's HTTP client has no timeouts** (gdrive.rs:25), which the calendar's own file header
   identified as a bug in 2026 and never fixed upstream.
5. **margin's `gdrive_disconnect` revokes the suite-wide grant silently** (gdrive.rs:776-803) where
   Margin Mail names the consequence before offering the button (auth.rs:1026-1029).
6. **`chacha20poly1305` 0.10 against 0.11** between the calendar and mail, with no on-disk format
   difference. Any shared crate should be on 0.11.
7. **`reqwest` 0.12 against 0.13**, which has already produced three hand-written helpers in Margin
   Mail that the other two get from the library.

## What I would extract, and what I would not

**Extract, high confidence.** A `margin-google-auth` crate holding the PKCE flow, the loopback
listener, the deep-link path, `browser.rs` whole, the credentials loader, the build script, the token
endpoint calls, the session map and `valid_access_token`. That is around 1,550 lines that currently
exist twice verbatim, plus roughly 330 more in `gdrive.rs` written independently and worse. It needs
these as parameters rather than constants: the scope list and the required-scope rule, the app name
for the listener page and the "not set up" sentence, the Android package scheme, and a small trait
for "record this account", since one app writes a SQLite row and the other writes a JSON file.
Whether `AuthEvent` gains mail's two extra fields for everyone or stays app-shaped is a taste call;
mail's shape is a superset and costs the calendar two empty vectors.

**Extract, high confidence.** A sealed-secrets module from `google/secrets.rs`, on
chacha20poly1305 0.11, taking `SERVICE` and `KEY_CONTEXT` as parameters. 326 lines that exist twice
and are already being used by Margin Mail as a general key-value store rather than an OAuth-specific
one.

**Extract, worth doing for the fix rather than the deduplication.** The Google error and retry layer
from Margin Mail's `google/api.rs`. The calendar needs it and has nothing.

**Extract, small and easy.** The five Drive verbs, the `margin` root folder constant, `app_data_dir`
and `atomic_write`, and the `meta`-table schema migrator. None of these is large; all of them exist
two or three times.

**Do not extract.** The sync engine. Sharing the poll-loop scaffolding, the `Sink` trait and the
connection-borrowing seam is defensible and would come to a couple of hundred lines. Sharing
anything below that would mean building an abstraction over "a token per collection committed at
the end of a chain, with etags" and "a log per account committed halfway, with declarative writes and
a quota", and the abstraction would be larger and harder to read than either of the two engines it
replaced.
