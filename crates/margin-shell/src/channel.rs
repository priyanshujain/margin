use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Runtime};

/// The package manager that installed this build, from an env var the Nix or Homebrew wrapper sets.
pub fn packaged_by(env_var: &str) -> Option<String> {
    std::env::var(env_var)
        .ok()
        .filter(|manager| !manager.is_empty())
}

/// A Mac App Store receipt inside the bundle. A bundle carrying one must never self-update.
pub fn has_appstore_receipt() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::env::current_exe()
            .ok()
            .and_then(|path| {
                path.parent()?
                    .parent()
                    .map(|contents| contents.join("_MASReceipt/receipt"))
            })
            .is_some_and(|path| path.is_file())
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

fn has_updater(config: &tauri::Config) -> bool {
    config
        .plugins
        .0
        .get("updater")
        .is_some_and(|value| !value.is_null())
}

fn has_appstore(config: &tauri::Config) -> bool {
    config.plugins.0.contains_key("appstore")
}

/// `"appstore"`, `"direct"` or `"none"`. The App Store wins over an updater config.
pub fn decide(updater: bool, appstore: bool, receipt: bool) -> &'static str {
    if receipt || appstore {
        "appstore"
    } else if updater {
        "direct"
    } else {
        "none"
    }
}

pub fn channel(config: &tauri::Config) -> &'static str {
    decide(
        has_updater(config),
        has_appstore(config),
        has_appstore_receipt(),
    )
}

/// Whether the updater plugin should be registered at all.
pub fn wants_updater(config: &tauri::Config) -> bool {
    channel(config) == "direct"
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStoreRelease {
    pub version: String,
    pub track_id: u64,
}

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// The newer App Store release, if any, from Apple's lookup endpoint. The timestamp gets past its
/// edge cache.
pub async fn appstore_latest(
    bundle_id: &str,
    current: &semver::Version,
) -> Result<Option<AppStoreRelease>, String> {
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
    Ok(newer_release(&body, current))
}

fn newer_release(body: &serde_json::Value, current: &semver::Version) -> Option<AppStoreRelease> {
    let result = body["results"].get(0)?;
    let version = result["version"].as_str()?;
    let track_id = result["trackId"].as_u64()?;
    let latest = semver::Version::parse(version).ok()?;
    (latest > *current).then(|| AppStoreRelease {
        version: version.to_string(),
        track_id,
    })
}

pub mod commands {
    use super::*;

    #[tauri::command]
    pub fn update_channel<R: Runtime>(app: AppHandle<R>) -> &'static str {
        channel(app.config())
    }

    #[tauri::command]
    pub async fn appstore_latest<R: Runtime>(
        app: AppHandle<R>,
    ) -> Result<Option<AppStoreRelease>, String> {
        let bundle_id = app.config().identifier.clone();
        let current = app.package_info().version.clone();
        super::appstore_latest(&bundle_id, &current).await
    }

    #[tauri::command]
    pub fn open_appstore(track_id: u64) -> Result<(), String> {
        let url = format!("macappstore://apps.apple.com/app/id{track_id}");
        tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn open_appstore_updates() -> Result<(), String> {
        tauri_plugin_opener::open_url("macappstore://showUpdatesPage", None::<&str>)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_store_wins_over_an_updater_config() {
        assert_eq!(decide(true, false, false), "direct");
        assert_eq!(decide(true, true, false), "appstore");
        assert_eq!(decide(true, false, true), "appstore");
        assert_eq!(decide(false, false, false), "none");
    }

    #[test]
    fn only_a_strictly_newer_release_is_offered() {
        let current = semver::Version::new(1, 2, 0);
        let body = |v: &str| serde_json::json!({"results":[{"version": v, "trackId": 42}]});
        let newer = newer_release(&body("1.3.0"), &current).expect("newer");
        assert_eq!((newer.version.as_str(), newer.track_id), ("1.3.0", 42));
        assert!(newer_release(&body("1.2.0"), &current).is_none());
        assert!(newer_release(&body("junk"), &current).is_none());
        assert!(newer_release(&serde_json::json!({"results":[]}), &current).is_none());
    }
}
