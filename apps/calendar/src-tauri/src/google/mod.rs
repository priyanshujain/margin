// auth.rs      sign-in and token refresh on margin-oauth, with Google as margin-google's configuration
// api.rs       typed Google Calendar REST wrapper on margin-http
// calendar.rs  Google as a `CalendarProvider`, and the only code that knows Google's shapes

pub mod api;
pub mod auth;
pub mod calendar;

use crate::dto::Account;

pub use auth::{AuthState, list_accounts};

#[tauri::command]
pub async fn accounts_list(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthState>,
) -> Result<Vec<Account>, String> {
    auth::list_accounts(&app, &state).await
}

/// Returns the consent URL. Completion arrives on the frontend as the `auth` event.
#[tauri::command]
pub async fn account_connect(app: tauri::AppHandle) -> Result<String, String> {
    auth::connect(app).await
}

/// Stops the sign-in in flight, for an add-account sheet closed before the browser answered.
#[tauri::command]
pub async fn account_connect_cancel(app: tauri::AppHandle) -> Result<(), String> {
    auth::cancel(&app).await;
    Ok(())
}

/// Whatever the account's provider, it leaves this device the way that provider needs.
#[tauri::command]
pub async fn account_disconnect(
    app: tauri::AppHandle,
    state: tauri::State<'_, AuthState>,
    account_id: String,
) -> Result<(), String> {
    let accounts = list_accounts(&app, &state).await?;
    match accounts.iter().find(|a| a.id == account_id).map(|a| a.provider.as_str()) {
        Some(crate::caldav::PROVIDER) => crate::caldav::disconnect(&app, &account_id),
        _ => auth::disconnect(&app, &state, &account_id).await,
    }
}
