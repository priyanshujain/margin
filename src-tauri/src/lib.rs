mod epub;
mod fonts;
mod gdrive;
mod library;
mod pdf;
mod project;
mod proofing;

use tauri::menu::{Menu, MenuItemBuilder, MenuItemKind, PredefinedMenuItem, SubmenuBuilder};
use tauri::{Emitter, Runtime};

fn build_menu<R: Runtime>(handle: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(handle)?;
    let new_book = MenuItemBuilder::with_id("new-book", "New Book")
        .accelerator("CmdOrCtrl+N")
        .build(handle)?;
    let export_pdf = MenuItemBuilder::with_id("export-pdf", "Export as PDF…").build(handle)?;
    let export_epub = MenuItemBuilder::with_id("export-epub", "Export as EPUB…").build(handle)?;
    let check_updates =
        MenuItemBuilder::with_id("check-updates", "Check for Updates…").build(handle)?;

    let file_submenu = menu
        .items()?
        .into_iter()
        .find_map(|item| match item {
            MenuItemKind::Submenu(submenu)
                if submenu.text().map(|t| t == "File").unwrap_or(false) =>
            {
                Some(submenu)
            }
            _ => None,
        });

    match file_submenu {
        Some(submenu) => {
            submenu.prepend_items(&[
                &new_book,
                &PredefinedMenuItem::separator(handle)?,
                &export_pdf,
                &export_epub,
                &PredefinedMenuItem::separator(handle)?,
                &check_updates,
                &PredefinedMenuItem::separator(handle)?,
            ])?;
        }
        None => {
            let submenu = SubmenuBuilder::new(handle, "File")
                .item(&new_book)
                .item(&PredefinedMenuItem::separator(handle)?)
                .item(&export_pdf)
                .item(&export_epub)
                .item(&PredefinedMenuItem::separator(handle)?)
                .item(&check_updates)
                .build()?;
            menu.insert(&submenu, 1)?;
        }
    }
    Ok(menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init());

    if context.config().plugins.0.contains_key("updater") {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .manage(proofing::new_state())
        .manage(gdrive::GDriveState::default())
        .setup(|app| {
            gdrive::init_session(app.handle());
            Ok(())
        })
        .menu(|handle| build_menu(handle))
        .on_menu_event(|app, event| {
            if matches!(
                event.id().0.as_str(),
                "new-book" | "export-pdf" | "export-epub" | "check-updates"
            ) {
                app.emit("menu-action", event.id().0.as_str()).ok();
            }
        })
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
            gdrive::gdrive_connect,
            gdrive::gdrive_disconnect,
            gdrive::gdrive_status,
            gdrive::gdrive_backup,
            gdrive::gdrive_restore,
            gdrive::gdrive_list_backups
        ])
        .run(context)
        .expect("error while running margin");
}
