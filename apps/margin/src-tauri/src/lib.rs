pub mod epub;
pub mod gdrive;
pub mod library;
pub mod pdf;
pub mod project;
pub mod proofing;
mod writingtools;

#[cfg(desktop)]
use tauri::Runtime;
#[cfg(desktop)]
use tauri::menu::Menu;

#[cfg(desktop)]
const ROUTED_MENU_IDS: [&str; 15] = [
    "new-book",
    "save",
    "export-pdf",
    "export-epub",
    "command-palette",
    "check-updates",
    "settings",
    "find",
    "toggle-chapters",
    "next-chapter",
    "prev-chapter",
    "report-issue",
    "keyboard-shortcuts",
    "writing-proofread",
    "writing-rewrite",
];

#[cfg(desktop)]
fn build_menu<R: Runtime>(handle: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    use margin_shell::menu::{Standard, file, find, item, separator};

    let menu = Menu::default(handle)?;
    let standard = Standard::new(
        handle,
        margin_shell::channel::channel(handle.config()) != "none",
    )?;

    let file_menu = file(
        handle,
        &menu,
        &[
            &item(handle, "new-book", "New Project", Some("CmdOrCtrl+N"))?,
            &separator(handle)?,
            &item(
                handle,
                "command-palette",
                "Command Palette…",
                Some("CmdOrCtrl+Shift+P"),
            )?,
            &separator(handle)?,
            &item(handle, "save", "Save", Some("CmdOrCtrl+S"))?,
            &separator(handle)?,
            &item(
                handle,
                "export-pdf",
                "Export as PDF…",
                Some("CmdOrCtrl+Alt+P"),
            )?,
            &item(
                handle,
                "export-epub",
                "Export as EPUB…",
                Some("CmdOrCtrl+Shift+E"),
            )?,
            &separator(handle)?,
        ],
    )?;

    if let Some(edit) = find(&menu, "Edit")? {
        edit.append_items(&[
            &separator(handle)?,
            &item(handle, "find", "Find…", Some("CmdOrCtrl+F"))?,
        ])?;
        #[cfg(target_os = "macos")]
        edit.append_items(&[
            &separator(handle)?,
            &item(
                handle,
                "writing-proofread",
                "Proofread",
                Some("Shift+Alt+F"),
            )?,
            &item(handle, "writing-rewrite", "Rewrite", Some("Shift+Alt+R"))?,
        ])?;
    }

    let shortcuts = tauri::menu::SubmenuBuilder::new(handle, "Shortcuts")
        .item(&item(
            handle,
            "keyboard-shortcuts",
            "Keyboard Shortcuts…",
            Some("CmdOrCtrl+/"),
        )?)
        .build()?;
    menu.append(&shortcuts)?;

    if let Some(help) = find(&menu, "Help")? {
        help.append_items(&[&standard.report_issue])?;
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(window) = find(&menu, "Window")? {
            window.append_items(&[
                &separator(handle)?,
                &item(
                    handle,
                    "show-window",
                    "Open Window",
                    Some("CmdOrCtrl+Shift+M"),
                )?,
            ])?;
        }
        if let Some(view) = find(&menu, "View")? {
            view.prepend_items(&[
                &item(
                    handle,
                    "toggle-chapters",
                    "Toggle Chapters",
                    Some("CmdOrCtrl+\\"),
                )?,
                &separator(handle)?,
                &item(
                    handle,
                    "next-chapter",
                    "Next Chapter",
                    Some("CmdOrCtrl+Alt+Down"),
                )?,
                &item(
                    handle,
                    "prev-chapter",
                    "Previous Chapter",
                    Some("CmdOrCtrl+Alt+Up"),
                )?,
                &separator(handle)?,
            ])?;
        }
    }

    standard.place(handle, &menu, &file_menu)?;
    Ok(menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    let mut builder = margin_shell::plugins::standard(tauri::Builder::default(), context.config())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(proofing::CustomWords::default())
        .manage(gdrive::GDriveState::default())
        .manage(project::GrantedPaths::default())
        .setup(|app| {
            match margin_shell::dirs::app_data_dir(app.handle()) {
                Ok(dir) => margin_log::init(&dir, "margin.log"),
                Err(e) => eprintln!("failed to prepare app data dir: {e}"),
            }
            gdrive::init_session(app.handle());
            proofing::install(app.handle());
            margin_mac::writing_tools::install();
            Ok(())
        });

    #[cfg(desktop)]
    {
        builder = builder.menu(build_menu).on_menu_event(|app, event| {
            let id = event.id().0.as_str();
            #[cfg(target_os = "macos")]
            if id == "show-window" {
                margin_shell::window::show_or_create(app, "main");
                return;
            }
            margin_shell::menu::forward(app, id, &ROUTED_MENU_IDS);
        });
    }

    let app = builder
        .invoke_handler(tauri::generate_handler![
            epub::package_epub,
            epub::unzip_epub,
            pdf::list_system_fonts,
            pdf::compile_pdf,
            project::pick_save_path,
            project::pick_open_path,
            project::read_file,
            project::write_file,
            project::write_bytes,
            library::list_books,
            library::load_book,
            library::save_book,
            library::delete_book,
            proofing::spell_check,
            proofing::spell_learn,
            proofing::spell_unlearn,
            proofing::spell_available,
            proofing::grammar_check,
            proofing::grammar_available,
            writingtools::writing_available,
            writingtools::run_writing_tool,
            margin_log::commands::log_note,
            gdrive::gdrive_connect,
            gdrive::gdrive_connect_cancel,
            gdrive::gdrive_disconnect,
            gdrive::gdrive_status,
            gdrive::gdrive_backup,
            gdrive::gdrive_sync,
            gdrive::gdrive_restore,
            gdrive::gdrive_list_backups,
            margin_shell::channel::commands::update_channel,
            margin_shell::channel::commands::appstore_latest,
            margin_shell::channel::commands::open_appstore
        ])
        .build(context)
        .expect("error while building margin");

    app.run(|_app, _event| {
        #[cfg(target_os = "macos")]
        match &_event {
            tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } => api.prevent_exit(),
            tauri::RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } => margin_shell::window::show_or_create(_app, "main"),
            _ => {}
        }
    });
}
