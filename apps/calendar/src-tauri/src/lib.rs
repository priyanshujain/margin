mod caldav;
mod dto;
mod google;
mod ical;
mod local;
mod provider;
mod recur;
mod secrets;
mod store;
mod subscribe;
mod sync;

#[cfg(desktop)]
use margin_shell::menu;
#[cfg(desktop)]
use tauri::menu::Menu;
#[cfg(desktop)]
use tauri::Runtime;
use tauri::{Emitter, Manager};

/// The frontend treats this purely as an invalidation signal and re-requests the visible range.
pub fn emit_store_changed(app: &tauri::AppHandle, reason: &str) {
    let _ = app.emit("store-changed", reason);
}

/// Exactly the ids `src/keys/bindings.ts` lists as `MENU_COMMAND_IDS`.
#[cfg(desktop)]
const MENU_ACTIONS: &[&str] = &[
    "new-event",
    "command-palette",
    "sync-now",
    "accounts",
    "check-updates",
    "settings",
    "search",
    "today",
    "shortcuts",
    "view-day",
    "view-week",
    "view-agenda",
    "report-issue",
];

#[cfg(desktop)]
fn build_menu<R: Runtime>(handle: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(handle)?;
    let standard = menu::Standard::new(handle, true)?;

    let new_event = menu::item(handle, "new-event", "New Event", Some("CmdOrCtrl+N"))?;
    let command_palette =
        menu::item(handle, "command-palette", "Command Palette…", Some("CmdOrCtrl+K"))?;
    let sync_now = menu::item(handle, "sync-now", "Sync Now", Some("CmdOrCtrl+R"))?;
    let accounts = menu::item(handle, "accounts", "Accounts…", None)?;
    let search = menu::item(handle, "search", "Search…", Some("CmdOrCtrl+F"))?;
    let shortcuts = menu::item(handle, "shortcuts", "Keyboard Shortcuts", Some("CmdOrCtrl+/"))?;

    let file = menu::file(
        handle,
        &menu,
        &[
            &new_event,
            &command_palette,
            &menu::separator(handle)?,
            &sync_now,
            &accounts,
            &menu::separator(handle)?,
        ],
    )?;

    if let Some(edit) = menu::find(&menu, "Edit")? {
        edit.append_items(&[&menu::separator(handle)?, &search])?;
    }

    if let Some(help) = menu::find(&menu, "Help")? {
        help.append_items(&[&shortcuts, &standard.report_issue])?;
    }

    standard.place(handle, &menu, &file)?;

    #[cfg(target_os = "macos")]
    if let Some(view) = menu::find(&menu, "View")? {
        let today = menu::item(handle, "today", "Today", Some("CmdOrCtrl+T"))?;
        let view_day = menu::item(handle, "view-day", "Day", Some("CmdOrCtrl+1"))?;
        let view_week = menu::item(handle, "view-week", "Week", Some("CmdOrCtrl+2"))?;
        let view_agenda = menu::item(handle, "view-agenda", "Agenda", Some("CmdOrCtrl+3"))?;
        view.prepend_items(&[
            &view_day,
            &view_week,
            &view_agenda,
            &menu::separator(handle)?,
            &today,
            &menu::separator(handle)?,
        ])?;
    }

    Ok(menu)
}

/// The deep link half of mobile auth: Google's answer arrives as a link into this app rather than
/// on a loopback socket. Only the flow that runs when a phone has its own OAuth client uses it, but
/// it stays registered on both platforms either way, because the OS has to be told about a scheme
/// at install time and cannot be told about one later.
///
/// Both arrival routes are covered. `on_open_url` catches the link when the app was already
/// running, which is the usual case since it is what opened the browser a moment ago;
/// `get_current` catches the one that launched a process the OS had killed in the meantime.
/// Handling it twice is harmless: `handle_redirect` takes the pending verifier rather than reading
/// it, so the second call finds nothing waiting and returns.
#[cfg(mobile)]
fn listen_for_redirects(handle: &tauri::AppHandle) {
    use tauri_plugin_deep_link::DeepLinkExt;

    let dispatch = |handle: &tauri::AppHandle, url: url::Url| {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            google::auth::handle_redirect(handle, &url).await;
        });
    };

    if let Ok(Some(urls)) = handle.deep_link().get_current() {
        for url in urls {
            dispatch(handle, url);
        }
    }

    let handle = handle.clone();
    handle.clone().deep_link().on_open_url(move |event| {
        for url in event.urls() {
            dispatch(&handle, url);
        }
    });
}

/// UIKit hands a scroll view the safe areas as content insets unless it is told not to, and wry
/// never tells it not to: wry 0.55.1's src/wkwebview/mod.rs:528 reaches for the scroll view only to
/// switch `bounces` off. WebKit then lays the page out in what is left over. On an iPhone 17 Pro
/// that is a layout viewport 778pt tall against an 874pt screen, still anchored at y 0, so the
/// bottom 96pt of the display is outside the page altogether. `body` is `position: fixed`, which
/// clips to that box, so nothing can paint down there whatever the CSS says: the tab bar sits in
/// the middle of the glass with a dead band of shell colour beneath it.
///
/// `never` gives the page the whole screen back. The insets are not lost, they arrive as
/// `env(safe-area-inset-*)` instead, which is where the two bars in app.css already read them from,
/// and all four viewport units finally agree on 874.
#[cfg(target_os = "ios")]
fn stop_uikit_shrinking_the_viewport(window: &tauri::WebviewWindow) {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_ui_kit::{UIScrollView, UIScrollViewContentInsetAdjustmentBehavior};

    let _ = window.with_webview(|webview| {
        // `inner()` is the WKWebView, and `with_webview` runs the closure inline when the caller is
        // already on the main thread, which setup is. So this is a plain main-thread UIKit call.
        let scroll_view: Retained<UIScrollView> =
            unsafe { objc2::msg_send![webview.inner().cast::<AnyObject>(), scrollView] };
        scroll_view
            .setContentInsetAdjustmentBehavior(UIScrollViewContentInsetAdjustmentBehavior::Never);
    });
}

/// A Chrome Custom Tab is Chrome's activity sitting on top of ours, in our own task, so this
/// process really is backgrounded for the length of a consent round trip and coming back means the
/// tab has gone. That is the only notice Android gives that the user backed out of signing in, and
/// without it the accounts panel waits on an answer that is never coming. `note_resumed` works out
/// whether the tab went because the user closed it or because this app took it down a moment after
/// the code arrived.
///
/// Not `RunEvent::Resumed`, which sounds right and is not: tauri raises that one on every poll of
/// the event loop. tao's real mobile resume arrives here, per window.
///
/// iOS shows the consent page inside the app, never leaves the foreground, and gets the same
/// question answered by a delegate in `google/browser.rs` instead.
#[cfg(target_os = "android")]
fn watch_for_the_consent_tab_closing(window: &tauri::WebviewWindow) {
    window.on_window_event(|event| {
        if matches!(event, tauri::WindowEvent::Resumed) {
            margin_oauth::browser::note_resumed();
        }
    });
}

/// The package manager that owns this install, when one does. The Nix wrapper sets it to "nix":
/// the binary lives in a read-only store there, so the updater may announce a version but not
/// install it.
#[tauri::command]
fn packaged_by() -> Option<String> {
    margin_shell::channel::packaged_by("MARGIN_CALENDAR_PACKAGED_BY")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    #[cfg_attr(mobile, allow(unused_mut))]
    let mut builder = margin_shell::plugins::standard(tauri::Builder::default(), context.config())
        .plugin(tauri_plugin_deep_link::init());

    builder = builder
        .manage(google::AuthState::default())
        .manage(sync::SyncState::default())
        .setup(|app| {
            let handle = app.handle();
            if let Ok(dir) = margin_shell::dirs::app_data_dir(handle) {
                margin_log::init(&dir, "margin-calendar.log");
            }
            let store = store::Store::open(handle)?;
            app.manage(store);
            google::auth::init_sessions(handle);
            sync::start_loop(handle.clone());
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
        builder = builder.menu(build_menu).on_menu_event(|app, event| {
            menu::forward(app, event.id().0.as_str(), MENU_ACTIONS);
        });
    }

    #[cfg(target_os = "macos")]
    {
        builder = builder.on_window_event(margin_shell::window::hide_on_close);
    }

    let app = builder
        .invoke_handler(tauri::generate_handler![
            google::accounts_list,
            google::account_connect,
            google::account_connect_cancel,
            google::account_disconnect,
            store::calendars_list,
            store::calendar_set_selected,
            local::calendar_create,
            local::calendar_delete,
            subscribe::ics_subscribe,
            subscribe::ics_import,
            caldav::caldav_connect,
            recur::instances_range,
            sync::event_create,
            sync::event_update,
            sync::event_delete,
            sync::sync_now,
            sync::sync_status,
            sync::sync_flush,
            margin_log::commands::log_note,
            packaged_by
        ])
        .build(context)
        .expect("error while building Margin Calendar");

    app.run(|app, event| {
        margin_shell::window::reopen_shows(app, &event, "main");
    });
}
