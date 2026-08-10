#[cfg(target_os = "macos")]
mod mac {
    use objc2::rc::Retained;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
    use objc2_foundation::{NSArray, NSString};

    const SHORTCUTS: [(&str, &str); 2] = [("Proofread", "f"), ("Rewrite", "r")];

    fn submenu_named(items: &NSArray<NSMenuItem>, title: &str) -> Option<Retained<NSMenu>> {
        for i in 0..items.count() {
            let item = items.objectAtIndex(i);
            if item.title().to_string() == title {
                return item.submenu();
            }
        }
        None
    }

    fn edit_menu(mtm: MainThreadMarker) -> Option<Retained<NSMenu>> {
        let main = NSApplication::sharedApplication(mtm).mainMenu()?;
        for i in 0..main.numberOfItems() {
            let Some(item) = main.itemAtIndex(i) else { continue };
            let Some(submenu) = item.submenu() else { continue };
            if submenu.title().to_string() == "Edit" || item.title().to_string() == "Edit" {
                return Some(submenu);
            }
        }
        None
    }

    fn writing_tools_menu(mtm: MainThreadMarker) -> Option<Retained<NSMenu>> {
        submenu_named(&edit_menu(mtm)?.itemArray(), "Writing Tools")
    }

    pub fn label_shortcuts() {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let Some(menu) = writing_tools_menu(mtm) else { return };
        let items = menu.itemArray();
        for i in 0..items.count() {
            let item = items.objectAtIndex(i);
            let title = item.title().to_string();
            let Some((_, key)) = SHORTCUTS.iter().find(|(name, _)| *name == title) else { continue };
            item.setKeyEquivalent(&NSString::from_str(key));
            item.setKeyEquivalentModifierMask(
                NSEventModifierFlags::Shift | NSEventModifierFlags::Option,
            );
        }
    }

    pub fn perform(tool: &str) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let Some(menu) = writing_tools_menu(mtm) else { return };
        let items = menu.itemArray();
        for i in 0..items.count() {
            if items.objectAtIndex(i).title().to_string() == tool {
                menu.performActionForItemAtIndex(i as isize);
                return;
            }
        }
    }
}

pub fn label_writing_tool_shortcuts() {
    #[cfg(target_os = "macos")]
    mac::label_shortcuts();
}

#[tauri::command]
pub fn run_writing_tool(app: tauri::AppHandle, tool: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        app.run_on_main_thread(move || mac::perform(&tool))
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, tool);
        Ok(())
    }
}
