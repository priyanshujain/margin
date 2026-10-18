use tauri::{AppHandle, Manager, Runtime, Window, WindowEvent};

/// Close policy for single-window apps on macOS: closing hides, the app keeps running.
/// Pass to `Builder::on_window_event`. Does nothing elsewhere.
pub fn hide_on_close<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    #[cfg(target_os = "macos")]
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, event);
}

/// Brings `label` back: unminimised, shown and focused.
pub fn show<R: Runtime>(app: &AppHandle<R>, label: &str) -> bool {
    let Some(window) = app.get_webview_window(label) else {
        return false;
    };
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    true
}

/// `show`, or a fresh window from the first window in the config when `label` was destroyed.
pub fn show_or_create<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if show(app, label) {
        return;
    }
    let Some(config) = app.config().app.windows.first().cloned() else {
        return;
    };
    if let Ok(builder) = tauri::WebviewWindowBuilder::from_config(app, &config) {
        let _ = builder.build();
    }
}

/// Dock click with no visible window shows the main one. Call from `App::run`.
pub fn reopen_shows<R: Runtime>(app: &AppHandle<R>, event: &tauri::RunEvent, label: &str) {
    #[cfg(target_os = "macos")]
    if let tauri::RunEvent::Reopen {
        has_visible_windows: false,
        ..
    } = event
    {
        show(app, label);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, event, label);
}
