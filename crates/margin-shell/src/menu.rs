use tauri::menu::{
    IsMenuItem, Menu, MenuItem, MenuItemBuilder, MenuItemKind, PredefinedMenuItem, Submenu,
    SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Runtime};

pub fn item<R: Runtime>(
    handle: &AppHandle<R>,
    id: &str,
    label: &str,
    accelerator: Option<&str>,
) -> tauri::Result<MenuItem<R>> {
    let builder = MenuItemBuilder::with_id(id, label);
    match accelerator {
        Some(accelerator) => builder.accelerator(accelerator),
        None => builder,
    }
    .build(handle)
}

pub fn separator<R: Runtime>(handle: &AppHandle<R>) -> tauri::Result<PredefinedMenuItem<R>> {
    PredefinedMenuItem::separator(handle)
}

/// A top level submenu by its title, such as "File" or "Help".
pub fn find<R: Runtime>(menu: &Menu<R>, name: &str) -> tauri::Result<Option<Submenu<R>>> {
    Ok(menu.items()?.into_iter().find_map(|item| match item {
        MenuItemKind::Submenu(submenu) if submenu.text().map(|t| t == name).unwrap_or(false) => {
            Some(submenu)
        }
        _ => None,
    }))
}

/// Puts `items` at the top of File, creating File after the app menu (macOS) or first when the
/// default menu has none.
pub fn file<R: Runtime>(
    handle: &AppHandle<R>,
    menu: &Menu<R>,
    items: &[&dyn IsMenuItem<R>],
) -> tauri::Result<Submenu<R>> {
    if let Some(submenu) = find(menu, "File")? {
        submenu.prepend_items(items)?;
        return Ok(submenu);
    }
    let submenu = SubmenuBuilder::new(handle, "File").items(items).build()?;
    menu.insert(&submenu, if cfg!(target_os = "macos") { 1 } else { 0 })?;
    Ok(submenu)
}

/// A submenu after Edit, or at index 2 when there is no Edit.
pub fn insert_after_edit<R: Runtime>(menu: &Menu<R>, submenu: &Submenu<R>) -> tauri::Result<()> {
    let after_edit = menu
        .items()?
        .iter()
        .position(|item| match item {
            MenuItemKind::Submenu(submenu) => submenu.text().map(|t| t == "Edit").unwrap_or(false),
            _ => false,
        })
        .map_or(2, |index| index + 1);
    menu.insert(submenu, after_edit)
}

/// The items every app has, with the suite's ids, labels and accelerators.
pub struct Standard<R: Runtime> {
    pub check_updates: Option<MenuItem<R>>,
    pub settings: MenuItem<R>,
    pub report_issue: MenuItem<R>,
}

impl<R: Runtime> Standard<R> {
    /// `with_updates` is false for builds with no update channel, which then show no such item.
    pub fn new(handle: &AppHandle<R>, with_updates: bool) -> tauri::Result<Self> {
        Ok(Standard {
            check_updates: with_updates
                .then(|| item(handle, "check-updates", "Check for Updates…", None))
                .transpose()?,
            settings: item(handle, "settings", "Settings…", Some("CmdOrCtrl+,"))?,
            report_issue: item(handle, "report-issue", "Report an Issue…", None)?,
        })
    }

    /// Check for Updates and Settings where each platform expects them: after About in the app
    /// menu on macOS; at the end of File and of Edit elsewhere. Report an Issue is left to the
    /// caller, which places it in Help with its own items.
    pub fn place(
        &self,
        handle: &AppHandle<R>,
        menu: &Menu<R>,
        file: &Submenu<R>,
    ) -> tauri::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let _ = file;
            let app_submenu = menu.items()?.into_iter().find_map(|item| match item {
                MenuItemKind::Submenu(submenu) => Some(submenu),
                _ => None,
            });
            if let Some(app_submenu) = app_submenu {
                let mut settings_at = 2;
                if let Some(check_updates) = &self.check_updates {
                    app_submenu.insert(check_updates, 1)?;
                    settings_at += 1;
                }
                app_submenu.insert(&self.settings, settings_at)?;
                app_submenu.insert(&separator(handle)?, settings_at + 1)?;
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            if let Some(check_updates) = &self.check_updates {
                file.append_items(&[&separator(handle)?, check_updates])?;
            }
            if let Some(edit) = find(menu, "Edit")? {
                edit.append_items(&[&separator(handle)?, &self.settings])?;
            }
        }
        Ok(())
    }
}

/// Forwards a menu click to the frontend as `menu-action` when its id is one of `ids`.
pub fn forward<R: Runtime>(app: &AppHandle<R>, id: &str, ids: &[&str]) -> bool {
    if ids.contains(&id) {
        app.emit("menu-action", id).ok();
        return true;
    }
    false
}
