mod epub;
mod library;
mod pdf;
mod project;

use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{Emitter, Runtime};

fn build_menu<R: Runtime>(handle: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(handle)?;
    let new_book = MenuItemBuilder::with_id("new-book", "New Book")
        .accelerator("CmdOrCtrl+N")
        .build(handle)?;
    let export_pdf = MenuItemBuilder::with_id("export-pdf", "Export as PDF…").build(handle)?;
    let export_epub = MenuItemBuilder::with_id("export-epub", "Export as EPUB…").build(handle)?;
    let file_submenu = SubmenuBuilder::new(handle, "File")
        .item(&new_book)
        .item(&PredefinedMenuItem::separator(handle)?)
        .item(&export_pdf)
        .item(&export_epub)
        .build()?;
    menu.insert(&file_submenu, 1)?;
    Ok(menu)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .menu(|handle| build_menu(handle))
        .on_menu_event(|app, event| {
            if matches!(event.id().0.as_str(), "new-book" | "export-pdf" | "export-epub") {
                app.emit("menu-action", event.id().0.as_str()).ok();
            }
        })
        .invoke_handler(tauri::generate_handler![
            epub::package_epub,
            epub::unzip_epub,
            pdf::compile_pdf,
            project::read_file,
            project::write_file,
            project::write_bytes,
            library::list_books,
            library::load_book,
            library::save_book,
            library::delete_book
        ])
        .run(tauri::generate_context!())
        .expect("error while running margin");
}
