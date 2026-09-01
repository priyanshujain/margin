mod epub;
mod fonts;
mod gdrive;
mod library;
#[cfg(target_os = "macos")]
mod macspell;
mod pdf;
mod project;
mod proofing;
mod updates;
mod writingtools;

#[cfg(desktop)]
use tauri::menu::{Menu, MenuItemBuilder, MenuItemKind, PredefinedMenuItem, SubmenuBuilder};
#[cfg(desktop)]
use tauri::{Emitter, Runtime};

#[cfg(desktop)]
fn build_menu<R: Runtime>(handle: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(handle)?;

    let new_book = MenuItemBuilder::with_id("new-book", "New Project")
        .accelerator("CmdOrCtrl+N")
        .build(handle)?;
    let save = MenuItemBuilder::with_id("save", "Save")
        .accelerator("CmdOrCtrl+S")
        .build(handle)?;
    let export_pdf = MenuItemBuilder::with_id("export-pdf", "Export as PDF…")
        .accelerator("CmdOrCtrl+Shift+P")
        .build(handle)?;
    let export_epub = MenuItemBuilder::with_id("export-epub", "Export as EPUB…")
        .accelerator("CmdOrCtrl+Shift+E")
        .build(handle)?;
    let check_updates = (updates::channel(handle) != "none")
        .then(|| MenuItemBuilder::with_id("check-updates", "Check for Updates…").build(handle))
        .transpose()?;
    let settings = MenuItemBuilder::with_id("settings", "Settings…")
        .accelerator("CmdOrCtrl+,")
        .build(handle)?;
    let find = MenuItemBuilder::with_id("find", "Find…")
        .accelerator("CmdOrCtrl+F")
        .build(handle)?;
    let report_issue =
        MenuItemBuilder::with_id("report-issue", "Report an Issue…").build(handle)?;

    let submenus: Vec<_> = menu
        .items()?
        .into_iter()
        .filter_map(|item| match item {
            MenuItemKind::Submenu(submenu) => Some(submenu),
            _ => None,
        })
        .collect();

    let find_submenu = |name: &str| {
        submenus
            .iter()
            .find(|submenu| submenu.text().map(|t| t == name).unwrap_or(false))
            .cloned()
    };

    match find_submenu("File") {
        Some(submenu) => {
            submenu.prepend_items(&[
                &new_book,
                &PredefinedMenuItem::separator(handle)?,
                &save,
                &PredefinedMenuItem::separator(handle)?,
                &export_pdf,
                &export_epub,
                &PredefinedMenuItem::separator(handle)?,
            ])?;
        }
        None => {
            let submenu = SubmenuBuilder::new(handle, "File")
                .item(&new_book)
                .item(&PredefinedMenuItem::separator(handle)?)
                .item(&save)
                .item(&PredefinedMenuItem::separator(handle)?)
                .item(&export_pdf)
                .item(&export_epub)
                .build()?;
            menu.insert(&submenu, 1)?;
        }
    }

    if let Some(edit) = find_submenu("Edit") {
        edit.append_items(&[&PredefinedMenuItem::separator(handle)?, &find])?;
    }

    if let Some(help) = find_submenu("Help") {
        help.append_items(&[&report_issue])?;
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(app_submenu) = submenus.first() {
            let mut settings_at = 2;
            if let Some(check_updates) = &check_updates {
                app_submenu.insert(check_updates, 1)?;
                settings_at += 1;
            }
            app_submenu.insert(&settings, settings_at)?;
            app_submenu.insert(&PredefinedMenuItem::separator(handle)?, settings_at + 1)?;
        }
        if let Some(window) = find_submenu("Window") {
            let show_window = MenuItemBuilder::with_id("show-window", "Open Window")
                .accelerator("CmdOrCtrl+Shift+M")
                .build(handle)?;
            window.append_items(&[&PredefinedMenuItem::separator(handle)?, &show_window])?;
        }
        if let Some(view) = find_submenu("View") {
            let toggle_chapters = MenuItemBuilder::with_id("toggle-chapters", "Toggle Chapters")
                .accelerator("CmdOrCtrl+\\")
                .build(handle)?;
            let next_chapter = MenuItemBuilder::with_id("next-chapter", "Next Chapter")
                .accelerator("CmdOrCtrl+Alt+Down")
                .build(handle)?;
            let prev_chapter = MenuItemBuilder::with_id("prev-chapter", "Previous Chapter")
                .accelerator("CmdOrCtrl+Alt+Up")
                .build(handle)?;
            view.prepend_items(&[
                &toggle_chapters,
                &PredefinedMenuItem::separator(handle)?,
                &next_chapter,
                &prev_chapter,
                &PredefinedMenuItem::separator(handle)?,
            ])?;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Some(file) = find_submenu("File") {
            if let Some(check_updates) = &check_updates {
                file.append_items(&[&PredefinedMenuItem::separator(handle)?, check_updates])?;
            }
        }
        if let Some(edit) = find_submenu("Edit") {
            edit.append_items(&[&settings])?;
        }
    }

    Ok(menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    #[cfg_attr(mobile, allow(unused_mut))]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_process::init());
        if context.config().plugins.0.contains_key("updater") {
            builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
        }
    }

    builder = builder
        .manage(proofing::new_state())
        .manage(gdrive::GDriveState::default())
        .setup(|app| {
            gdrive::init_session(app.handle());
            writingtools::label_writing_tool_shortcuts();
            Ok(())
        });

    #[cfg(desktop)]
    {
        builder = builder
            .menu(|handle| build_menu(handle))
            .on_menu_event(|app, event| {
                if matches!(
                    event.id().0.as_str(),
                    "new-book"
                        | "save"
                        | "export-pdf"
                        | "export-epub"
                        | "check-updates"
                        | "settings"
                        | "find"
                        | "toggle-chapters"
                        | "next-chapter"
                        | "prev-chapter"
                        | "report-issue"
                ) {
                    app.emit("menu-action", event.id().0.as_str()).ok();
                }
                #[cfg(target_os = "macos")]
                if event.id().0.as_str() == "show-window" {
                    open_main_window(app);
                }
            });
    }

    let app = builder
        .invoke_handler(tauri::generate_handler![
            epub::package_epub,
            epub::unzip_epub,
            fonts::list_system_fonts,
            pdf::compile_pdf,
            project::read_file,
            project::write_file,
            project::write_bytes,
            library::list_books,
            library::load_book,
            library::save_book,
            library::delete_book,
            proofing::proof_text,
            proofing::remember_word,
            writingtools::run_writing_tool,
            gdrive::gdrive_connect,
            gdrive::gdrive_disconnect,
            gdrive::gdrive_status,
            gdrive::gdrive_backup,
            gdrive::gdrive_sync,
            gdrive::gdrive_restore,
            gdrive::gdrive_list_backups,
            updates::update_channel,
            updates::appstore_latest,
            updates::open_appstore
        ])
        .build(context)
        .expect("error while building margin");

    app.run(|_app, _event| {
        #[cfg(target_os = "macos")]
        match &_event {
            tauri::RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
            tauri::RunEvent::Reopen { has_visible_windows: false, .. } => open_main_window(_app),
            _ => {}
        }
    });
}

#[cfg(target_os = "macos")]
fn open_main_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let Some(config) = app.config().app.windows.first().cloned() else {
        return;
    };
    if let Ok(builder) = tauri::WebviewWindowBuilder::from_config(app, &config) {
        let _ = builder.build();
    }
}
