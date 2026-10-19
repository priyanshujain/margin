pub mod dto;
pub mod fonts;
pub mod fs;
pub mod grammar;
pub mod index;
pub mod integration;
pub mod pdf;
pub mod spell;
pub mod watch;
pub mod windows;
pub mod writingtools;

use std::sync::Mutex;

use crate::dto::RootInfo;

#[cfg(desktop)]
use tauri::menu::Menu;
#[cfg(desktop)]
use tauri::Runtime;

#[cfg(desktop)]
use tauri::Manager;

/// The folders open in any window, in the order they were opened. Which window holds which is
/// `windows::Windows`' business; a folder open in two windows is one entry here, one watcher and one
/// set of index rows.
///
/// This lives here rather than in either module because both need it and neither owns the other:
/// `fs` puts roots in and takes them out, `watch` only ever turns an id back into a path.
pub const TITLEBAR_HEIGHT: f64 = 46.0;

#[derive(Default)]
pub struct Roots(pub Mutex<Vec<RootInfo>>);

impl Roots {
    /// The absolute path of an open root. Every command that takes a `rootId` needs this before it
    /// can touch anything, and an id that is not open is an error rather than an empty result.
    pub fn path_for(&self, id: &str) -> Result<String, String> {
        let roots = self.0.lock().map_err(|e| e.to_string())?;
        roots
            .iter()
            .find(|root| root.id == id)
            .map(|root| root.path.clone())
            .ok_or_else(|| format!("no such root: {id}"))
    }

    pub fn paths_for(&self, ids: &[String]) -> Result<Vec<String>, String> {
        let roots = self.0.lock().map_err(|e| e.to_string())?;
        Ok(roots
            .iter()
            .filter(|root| ids.contains(&root.id))
            .map(|root| root.path.clone())
            .collect())
    }
}

/// Menu items whose action belongs to whichever window the user is in.
#[cfg(desktop)]
const ROUTED_MENU_IDS: [&str; 17] = [
    "open-folder",
    "new-doc",
    "new-folder",
    "save",
    "close-folder",
    "settings",
    "find",
    "find-in-files",
    "quick-open",
    "command-palette",
    "toggle-sidebar",
    "toggle-outline",
    "check-updates",
    "report-issue",
    "export-pdf",
    "writing-proofread",
    "writing-rewrite",
];

#[cfg(desktop)]
fn build_menu<R: Runtime>(handle: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    use margin_shell::menu::{file, find, item, separator, Standard};

    let menu = Menu::default(handle)?;
    let standard = Standard::new(handle, true)?;

    let file_menu = file(
        handle,
        &menu,
        &[
            &item(handle, "new-window", "New Window", Some("CmdOrCtrl+Shift+N"))?,
            &item(handle, "open-folder", "Open Folder…", Some("CmdOrCtrl+O"))?,
            &item(handle, "new-doc", "New Document", Some("CmdOrCtrl+N"))?,
            &item(handle, "new-folder", "New Folder", None)?,
            &separator(handle)?,
            &item(handle, "quick-open", "Quick Open…", Some("CmdOrCtrl+P"))?,
            &item(handle, "command-palette", "Command Palette…", Some("CmdOrCtrl+Shift+P"))?,
            &separator(handle)?,
            &item(handle, "save", "Save", Some("CmdOrCtrl+S"))?,
            &item(handle, "export-pdf", "Export as PDF…", Some("CmdOrCtrl+Shift+E"))?,
            &separator(handle)?,
            &item(handle, "close-folder", "Close Folder", None)?,
            &separator(handle)?,
        ],
    )?;

    if let Some(edit) = find(&menu, "Edit")? {
        edit.append_items(&[
            &separator(handle)?,
            &item(handle, "find", "Find…", Some("CmdOrCtrl+F"))?,
            &item(handle, "find-in-files", "Find in Files…", Some("CmdOrCtrl+Shift+F"))?,
        ])?;
        #[cfg(target_os = "macos")]
        edit.append_items(&[
            &separator(handle)?,
            &item(handle, "writing-proofread", "Proofread", Some("Shift+Alt+F"))?,
            &item(handle, "writing-rewrite", "Rewrite", Some("Shift+Alt+R"))?,
        ])?;
    }

    if let Some(help) = find(&menu, "Help")? {
        help.append_items(&[&standard.report_issue])?;
    }

    #[cfg(target_os = "macos")]
    if let Some(view) = find(&menu, "View")? {
        view.prepend_items(&[
            &item(handle, "toggle-sidebar", "Toggle Sidebar", Some("CmdOrCtrl+\\"))?,
            &item(handle, "toggle-outline", "Toggle Outline", Some("CmdOrCtrl+Shift+\\"))?,
            &separator(handle)?,
        ])?;
    }

    standard.place(handle, &menu, &file_menu)?;
    Ok(menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    let mut builder = margin_shell::plugins::standard(tauri::Builder::default(), context.config())
        .plugin(tauri_plugin_dialog::init())
        .manage(Roots::default())
        .manage(windows::Windows::default())
        .manage(watch::Watchers::default())
        .manage(index::Index::default());

    builder = builder.setup(|app| {
        match margin_shell::dirs::app_data_dir(app.handle()) {
            Ok(dir) => margin_log::init(&dir, "margin-docs.log"),
            Err(e) => eprintln!("failed to prepare app data dir: {e}"),
        }
        // The index is opened here rather than lazily on the first search, because opening it is
        // where a schema migration runs and a migration that fails should say so at launch rather
        // than the first time somebody presses Cmd+P. A failure is not fatal: the app is a text
        // editor with a broken search box, which is worth far more than a window that will not
        // open.
        if let Err(e) = index::open(app.handle()) {
            eprintln!("failed to open the search index: {e}");
        }
        // Whatever the last session left open is closed now, once, rather than by each window as
        // it loads, which would close folders another window had only just opened.
        fs::forget_stale_roots(app.handle());
        #[cfg(target_os = "macos")]
        windows::terminate::install(app.handle());
        // The Writing Tools submenu is AppKit's, not build_menu's: the system inserts it into Edit
        // on its own terms, so when it is there to label is writingtools.rs's problem and not this
        // file's. One call, whatever it decides to wait for.
        writingtools::install();
        // The traffic lights, which land seven pixels above everything else in a title bar this
        // tall until they are moved. Here rather than in the window's own setup because the window
        // is Tauri's and this is the first point in the launch that has a handle to it.
        // Every window after this one gets the same in windows.rs, which is what builds them.
        if let Some(window) = app.get_webview_window(windows::MAIN) {
            margin_shell::titlebar::install(&window, TITLEBAR_HEIGHT);
        }
        // Last, so the files that launched the app find the main window ready to take the first.
        windows::start(app.handle());
        Ok(())
    });

    #[cfg(desktop)]
    {
        builder = builder
            .menu(build_menu)
            .on_menu_event(|app, event| {
                let id = event.id().0.as_str();
                if id == "new-window" {
                    if let Err(e) = windows::create_window(app, Default::default()) {
                        eprintln!("could not open a window: {e}");
                    }
                } else if ROUTED_MENU_IDS.contains(&id) {
                    windows::dispatch_menu(app, id);
                }
            })
            .on_window_event(|window, event| match event {
                tauri::WindowEvent::Focused(true) => windows::focused(window.app_handle(), window.label()),
                tauri::WindowEvent::Destroyed => windows::destroyed(window.app_handle(), window.label()),
                _ => {}
            });
    }

    // The whole command surface, in the order dto.rs describes it. Registering a command is this
    // file's job alone: a module adds a body, never a line here.
    builder
        .invoke_handler(tauri::generate_handler![
            fs::roots_list,
            fs::root_open,
            fs::root_close,
            fs::tree_read,
            fs::sweep_documents,
            fs::reveal_in_finder,
            fs::open_external,
            fs::file_read,
            fs::file_bytes,
            fs::file_write,
            fs::file_create,
            fs::file_folder_create,
            fs::file_rename,
            fs::file_move,
            fs::file_duplicate,
            fs::file_trash,
            fs::asset_write,
            watch::watch_start,
            watch::watch_stop,
            fs::index_rebuild,
            fs::index_status,
            fs::search_quick_open,
            fs::search_text,
            fs::backlinks_for,
            spell::spell_check,
            spell::spell_learn,
            spell::spell_unlearn,
            spell::spell_available,
            writingtools::writing_available,
            writingtools::writing_run,
            pdf::pdf_compile,
            pdf::pdf_write,
            grammar::grammar_available,
            grammar::grammar_check,
            fonts::fonts_list_system,
            margin_log::commands::log_note,
            windows::window_init,
            windows::window_create,
            windows::window_close_refused,
            windows::document_claim,
            windows::document_set,
            windows::document_open,
            integration::cli_install,
            integration::default_app_set,
        ])
        .build(context)
        .expect("error while building Margin Docs")
        .run(|app, event| match event {
            // Finder, the Dock and `mdocs` all hand over files this way, on a cold launch and to an
            // app that is already running alike.
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            tauri::RunEvent::Opened { urls } => {
                let paths = urls
                    .into_iter()
                    .filter_map(|url| url.to_file_path().ok())
                    .collect();
                windows::open_urls(app, paths);
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } => windows::reopen(app, has_visible_windows),
            // The last window closing leaves a Mac app running, the way every other document app
            // does. Only a quit, which sets `quitting` first, lets it go.
            tauri::RunEvent::ExitRequested { code: None, api, .. } if !windows::quitting(app) => {
                api.prevent_exit();
            }
            _ => {}
        });
}
