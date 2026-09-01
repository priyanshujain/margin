use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Runtime};

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStoreRelease {
    version: String,
    track_id: u64,
}

// The build flavour decides this, but a Mac App Store receipt overrides it either way: a bundle
// carrying one must never self-update, whatever config it was built against.
pub fn channel<R: Runtime>(handle: &AppHandle<R>) -> &'static str {
    let plugins = &handle.config().plugins.0;
    if plugins.contains_key("updater") && !mas_receipt() {
        "direct"
    } else if mas_receipt() || plugins.contains_key("appstore") {
        "appstore"
    } else {
        "none"
    }
}

#[cfg(target_os = "macos")]
fn mas_receipt() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    exe.parent()
        .and_then(|macos| macos.parent())
        .map(|contents| contents.join("_MASReceipt").join("receipt").exists())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn mas_receipt() -> bool {
    false
}

#[tauri::command]
pub fn update_channel(app: AppHandle) -> &'static str {
    channel(&app)
}

// Apple's lookup endpoint is edge cached, so a release can take hours to show up here. The
// timestamp is the usual way past that.
#[tauri::command]
pub async fn appstore_latest(app: AppHandle) -> Result<Option<AppStoreRelease>, String> {
    let bundle_id = app.config().identifier.clone();
    let current = app.package_info().version.clone();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let url = format!("https://itunes.apple.com/lookup?bundleId={bundle_id}&t={stamp}");

    let body: serde_json::Value = HTTP
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let Some(result) = body["results"].get(0) else {
        return Ok(None);
    };
    let (Some(version), Some(track_id)) = (result["version"].as_str(), result["trackId"].as_u64())
    else {
        return Ok(None);
    };
    let Ok(latest) = semver::Version::parse(version) else {
        return Ok(None);
    };
    if latest <= current {
        return Ok(None);
    }
    Ok(Some(AppStoreRelease { version: version.to_string(), track_id }))
}

#[tauri::command]
pub fn open_appstore(track_id: u64) -> Result<(), String> {
    let url = format!("macappstore://apps.apple.com/app/id{track_id}");
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string())
}
