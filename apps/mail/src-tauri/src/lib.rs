// `pub` throughout rather than private modules: this crate is a library with a thin binary in
// front of it, the modules are the parts, and a contract type that nothing has consumed yet is a
// contract type rather than dead code.
pub mod accounts;
pub mod app_lock;
pub mod attachments;
pub mod imap;
pub mod backup;
pub mod badge;
pub mod clips;
pub mod contacts;
pub mod db;
pub mod decisions;
pub mod drafts;
pub mod dto;
pub mod exports;
#[cfg(test)]
pub mod fixtures;
pub mod google;
pub mod invites;
pub mod mime;
pub mod mirror;
pub mod notify;
pub mod moves;
pub mod piles;
pub mod provider;
pub mod proofing;
pub mod otp_autofill;
pub mod routing;
pub mod sanitize;
pub mod secrets;
pub mod screener;
pub mod send;
pub mod settings;
pub mod snooze;
pub mod state;
pub mod sync;
pub mod undo;
pub mod unsubscribe;
pub mod writingtools;

#[cfg(desktop)]
use margin_shell::menu;
#[cfg(desktop)]
use tauri::menu::Menu;
#[cfg(all(desktop, not(target_os = "macos")))]
use tauri::menu::SubmenuBuilder;
#[cfg(desktop)]
use tauri::Runtime;
use tauri::{Emitter, Manager};

/// The frontend treats this as an invalidation signal with a scope: the reason names what moved
/// (`threads`, `thread:<key>`, `accounts`, `settings`, `state`), so a note landing does not make
/// the list refetch its bodies. Mailspring's typed delta stream in one string.
///
/// The dock badge hangs off the same signal for the same reason: everything that can change what
/// is waiting in the Inbox already announces itself here, and a second notification path would be
/// a second thing to remember to call.
pub fn emit_store_changed(app: &tauri::AppHandle, reason: &str) {
    let _ = app.emit("store-changed", reason);
    badge::on_store_changed(app, reason);
}

/// The menu ids the frontend runs as commands, forwarded as `menu-action`.
#[cfg(desktop)]
const MENU_ACTIONS: &[&str] = &[
    "compose",
    "command-palette",
    "sync-now",
    "check-updates",
    "settings",
    "search",
    "shortcuts",
    "place-inbox",
    "place-feed",
    "place-paper-trail",
    "toggle-pane",
    "guide",
    "tour",
    "report-issue",
];

#[cfg(desktop)]
fn build_menu<R: Runtime>(handle: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(handle)?;
    let standard = menu::Standard::new(handle, true)?;

    let new_message = menu::item(handle, "compose", "New Message", Some("CmdOrCtrl+N"))?;
    let command_palette =
        menu::item(handle, "command-palette", "Command Palette…", Some("CmdOrCtrl+K"))?;
    let sync_now = menu::item(handle, "sync-now", "Sync Now", Some("CmdOrCtrl+R"))?;
    let search = menu::item(handle, "search", "Search…", Some("CmdOrCtrl+F"))?;
    let shortcuts = menu::item(handle, "shortcuts", "Keyboard Shortcuts", Some("CmdOrCtrl+/"))?;
    let guide = menu::item(handle, "guide", "Margin Mail Guide", None)?;
    let tour = menu::item(handle, "tour", "Take the Tour", None)?;

    let file = menu::file(
        handle,
        &menu,
        &[
            &new_message,
            &command_palette,
            &menu::separator(handle)?,
            &sync_now,
            &menu::separator(handle)?,
        ],
    )?;

    if let Some(edit) = menu::find(&menu, "Edit")? {
        edit.append_items(&[&menu::separator(handle)?, &search])?;
    }

    // The Help menu in the order macOS expects: what the app can tell you first, then the way to
    // tell somebody about the app.
    if let Some(help) = menu::find(&menu, "Help")? {
        help.append_items(&[
            &guide,
            &tour,
            &shortcuts,
            &menu::separator(handle)?,
            &standard.report_issue,
        ])?;
    }

    standard.place(handle, &menu, &file)?;

    let inbox = menu::item(handle, "place-inbox", "Inbox", Some("CmdOrCtrl+1"))?;
    let feed = menu::item(handle, "place-feed", "Feed", Some("CmdOrCtrl+2"))?;
    let paper_trail = menu::item(handle, "place-paper-trail", "Paper Trail", Some("CmdOrCtrl+3"))?;
    let toggle_pane = menu::item(handle, "toggle-pane", "Reading Pane", Some("CmdOrCtrl+\\"))?;

    // The three places and the reading pane, which belong under View on every platform. Only macOS
    // is given a View menu to put them in; everywhere else one is built here.
    #[cfg(target_os = "macos")]
    if let Some(view) = menu::find(&menu, "View")? {
        view.prepend_items(&[
            &inbox,
            &feed,
            &paper_trail,
            &menu::separator(handle)?,
            &toggle_pane,
            &menu::separator(handle)?,
        ])?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let view = SubmenuBuilder::new(handle, "View")
            .item(&inbox)
            .item(&feed)
            .item(&paper_trail)
            .item(&menu::separator(handle)?)
            .item(&toggle_pane)
            .build()?;
        menu::insert_after_edit(&menu, &view)?;
    }

    Ok(menu)
}

#[cfg(target_os = "macos")]
pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    margin_shell::window::show(app, "main");
}

/// The package manager that owns this install, when one does. The Nix wrapper sets it to "nix":
/// the binary lives in a read-only store there, so the updater may announce a version but not
/// install it.
#[tauri::command]
fn packaged_by() -> Option<String> {
    margin_shell::channel::packaged_by("MARGIN_MAIL_PACKAGED_BY")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Hands the engine the mailbox behind an account. The engine only ever speaks through the trait,
/// so this is the single place in the app that decides the mailbox behind an account is Gmail
/// rather than an IMAP server.
fn attach_account(handle: &tauri::AppHandle, account: &dto::Account) {
    match account.kind {
        dto::AccountKind::Google => {
            sync::attach(&account.id, google::gmail::Gmail::new(handle.clone(), &account.id));
        }
        // An IMAP account without its servers on disk is a half-written registry rather than an
        // account, so it is left unattached and says so in the account chip.
        dto::AccountKind::Imap => {
            if let Ok(Some(entry)) = accounts::find(handle, &account.id)
                && let Some(servers) = entry.servers {
                    sync::attach(
                        &account.id,
                        imap::provider::Imap::new(handle.clone(), &account.id, servers),
                    );
                }
        }
    }
}

/// The step between an account being written and its mail arriving: the window it is to hold,
/// then the provider behind it, then the first pass, now rather than at the next tick.
///
/// The window comes first because the first sync reads it, and it is asked for rather than
/// assumed because a month is a guess and a year of a busy mailbox is a long wait nobody was
/// told about. Until this is called the account is in the registry and nowhere else; a launch in
/// between attaches it with whatever window the settings hold, which is the default.
#[tauri::command]
async fn account_start(
    app: tauri::AppHandle,
    account_id: String,
    window_days: u32,
) -> Result<(), String> {
    settings::set_window_days(&app, &account_id, window_days)?;
    let account = accounts::list(&app)?
        .into_iter()
        .find(|account| account.id == account_id)
        .ok_or_else(|| format!("Account {account_id} is not connected."))?;
    attach_account(&app, &account);
    sync::kick(&app, &account_id);
    Ok(())
}

pub fn run() {
    // generate_context! first, so the updater plugin registers only when the merged config actually
    // has an `updater` key. Ported from margin's lib.rs.
    let context = tauri::generate_context!();

    #[cfg_attr(mobile, allow(unused_mut))]
    let mut builder = margin_shell::plugins::standard(tauri::Builder::default(), context.config())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_deep_link::init());

    builder = builder.manage(google::AuthState::default()).setup(|app| {
        app.manage(app_lock::AppLock::new(settings::load(app.handle())?.app_lock_enabled));
        let handle = app.handle();
        tauri::async_runtime::spawn_blocking(|| { let _ = otp_autofill::clear(); });

        // The mirror and the state database, one connection per account, opened lazily by the
        // first read. Managed here because every command that touches SQLite reaches for it.
        app.manage(db::Db::open(handle)?);

        // Start fresh is reversible for seven days, which is longer than this process lives, so its
        // records are on disk and this is what puts a surviving one back within reach of `z`.
        let _ = mirror::fresh::rearm(handle.state::<db::Db>().inner());

        // Restores a session per sealed token, so a launch does not send anybody back to consent.
        google::auth::init_sessions(handle);

        // The certificates somebody has already decided about, which is what stops a local bridge
        // asking the same question every restart.
        if let Ok(dir) = margin_shell::dirs::app_data_dir(handle) {
            imap::init(dir);
        }

        // One provider per connected account, then the poll loop. An account added while the app
        // is up joins through `account_start`, once the window it is to hold has been chosen.
        for account in accounts::list(handle).unwrap_or_default() {
            attach_account(handle, &account);
        }
        sync::setup(handle.clone());

        // The dock keeps whatever number it was left with across a quit, so a launch has to say
        // what is true now rather than waiting for the first thing to change.
        badge::refresh(handle);

        // The notification centre wants its delegate before anything is posted, and a click on a
        // notification comes back through it.
        #[cfg(target_os = "macos")]
        notify::macos::install(handle.clone());

        #[cfg(mobile)]
        listen_for_redirects(handle);
        #[cfg(target_os = "ios")]
        if let Some(window) = handle.get_webview_window("main") {
            stop_uikit_shrinking_the_viewport(&window);
        }
        #[cfg(target_os = "android")]
        if let Some(window) = handle.get_webview_window("main") {
            watch_for_the_consent_tab_closing(&window);
        }
        Ok(())
    });

    #[cfg(desktop)]
    {
        builder = builder
            .menu(build_menu)
            .on_menu_event(|app, event| {
                menu::forward(app, event.id().0.as_str(), MENU_ACTIONS);
            });
    }

    #[cfg(target_os = "macos")]
    {
        builder = builder.on_window_event(margin_shell::window::hide_on_close);
    }

    let app = builder
        .invoke_handler({
            let handler: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> = Box::new(tauri::generate_handler![
            app_lock::app_lock_status,
            app_lock::app_unlock,
            proofing::proof_text,
            otp_autofill::otp_autofill_status,
            otp_autofill::otp_autofill_enable,
            writingtools::writing_tools_available,
            writingtools::run_writing_tool,
            // Accounts and consent
            accounts::accounts_list,
            account_start,
            margin_shell::channel::commands::update_channel,
            margin_shell::channel::commands::open_appstore_updates,
            margin_log::commands::log_note,
            accounts::account_set_color,
            accounts::account_set_name,
            google::account_connect,
            google::account_connect_cancel,
            google::account_grant,
            google::account_remove,
            // Reading
            mirror::threads_list,
            mirror::place_counts,
            mirror::place_visited,
            imap::imap_discover,
            imap::imap_test,
            imap::imap_connect,
            imap::imap_servers,
            imap::imap_trust_cert,
            imap::imap_forget_cert,
            attachments::message_show_images,
            attachments::message_images_set,
            attachments::attachment_data_url,
            attachments::attachment_save,
            attachments::attachment_open,
            mirror::thread_view,
            mirror::thread_opened,
            mirror::thread_hydrate,
            mirror::labels_list,
            mirror::storage_used,
            // Triage
            mirror::flags_set,
            mirror::mark_all_seen,
            mirror::label_apply,
            mirror::label_move,
            mirror::mirror_clear,
            mirror::fresh::start_fresh,
            undo::undo_last,
            undo::undo_token,
            // Routing and the Screener
            screener::screener_list,
            screener::screener_decide,
            screener::screener_clear_all,
            screener::screener_seed,
            // The piles and their friends
            piles::pile_toggle,
            moves::thread_move,
            snooze::snooze_set,
            snooze::snooze_clear,
            snooze::snooze_evaluate,
            decisions::note_add,
            decisions::note_update,
            decisions::note_delete,
            decisions::thread_rename,
            decisions::thread_merge,
            decisions::thread_unmerge,
            decisions::thread_ignore,
            decisions::thread_notify,
            clips::clip_save,
            clips::clips_list,
            clips::clip_delete,
            clips::files_list,
            unsubscribe::unsubscribe,
            // Contacts
            contacts::contact_card,
            contacts::contact_update,
            contacts::contacts_list,
            contacts::contacts_suggest,
            // Writing
            drafts::draft_save,
            drafts::draft_attachment_store,
            drafts::draft_get,
            drafts::draft_list,
            drafts::draft_import,
            drafts::draft_delete,
            send::send,
            send::send_now,
            send::outbox_list,
            invites::invite_respond,
            // Backup
            backup::backup_status,
            backup::backup_configure,
            backup::backup_now,
            backup::backup_phrase,
            backup::backup_restore,
            // Sync
            sync::sync_now,
            sync::sync_status,
            sync::sync_flush,
            sync::sync_backfill,
            sync::search,
            sync::search_provider,
            // Settings
            settings::settings_get,
            settings::settings_set,
            settings::system_fonts,
            notify::notify_test,
            notify::notify_permission,
            notify::notify_request,
            notify::notify_open_settings,
            notify::notify_take,

            // Taking it all out again
            exports::export_mbox,
            exports::export_state,
            exports::import_state,
            exports::keymap_path,
            exports::keymap_reset,
            packaged_by
            ]);
            move |invoke| {
                if !app_lock::permits(&invoke) {
                    invoke.resolver.reject("Unlock your mailbox first.");
                    return true;
                }
                handler(invoke)
            }
        })
        .build(context)
        .expect("error while building Margin Mail");

    app.run(|app, event| {
        margin_shell::window::reopen_shows(app, &event, "main");
    });
}
