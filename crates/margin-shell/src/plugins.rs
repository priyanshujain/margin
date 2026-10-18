use tauri::{Builder, Runtime};

/// The plugins every app registers: opener everywhere, and on desktop process plus the updater
/// when this build updates itself (see `channel::wants_updater`).
pub fn standard<R: Runtime>(builder: Builder<R>, config: &tauri::Config) -> Builder<R> {
    let builder = builder.plugin(tauri_plugin_opener::init());
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = {
        let builder = builder.plugin(tauri_plugin_process::init());
        if crate::channel::wants_updater(config) {
            builder.plugin(tauri_plugin_updater::Builder::new().build())
        } else {
            builder
        }
    };
    #[cfg(any(target_os = "android", target_os = "ios"))]
    let _ = config;
    builder
}
