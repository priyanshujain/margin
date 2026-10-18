//! Pieces every Margin app's `lib.rs` used to write for itself. Each is opt in; nothing here owns
//! the builder or the setup hook.

pub mod channel;
pub mod dirs;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod menu;
pub mod plugins;
pub mod titlebar;
pub mod window;

/// The App Group every Apple App Store build shares suite configuration through.
pub const APP_GROUP: &str = "group.studio.margin";
