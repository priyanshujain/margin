// Google Drive's scope for Mail. The store itself is objstore-gdrive, reached through margin-sync;
// it writes into the visible `margin` folder at the root of the user's Drive, under `mail/`, the
// same place Mail's backups always went, and `drive.file` means it sees only files it created.

pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";

/// The suite's one folder, at the root of the user's Drive and visible in it.
pub const FOLDER_NAME: &str = "margin";

/// This app's corner of it. The sibling apps use their own.
pub const MAIL_FOLDER: &str = "mail";

/// Said in the user's words rather than the API's, because "insufficient authentication scopes" is
/// not a sentence anybody can act on. The wording matches the tick box on Google's consent screen.
pub const REAUTH_MESSAGE: &str = "Margin Mail needs the \"See, edit, create and delete only the specific Google Drive files you use with this app\" permission to sync. Connect again and tick that box.";

/// Granular consent means the tick box is the user's to clear, so the granted scope list is checked
/// before the first call rather than a 403 being explained after it.
pub fn has_scope(granted: &[String]) -> bool {
    granted.iter().any(|scope| scope == SCOPE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_tick_box_is_caught_before_the_call_rather_than_explained_after_it() {
        let granted = vec![
            "openid".to_string(),
            "https://www.googleapis.com/auth/gmail.modify".to_string(),
        ];
        assert!(!has_scope(&granted));

        let mut granted = granted;
        granted.push(SCOPE.to_string());
        assert!(has_scope(&granted));
    }

    #[test]
    fn the_reauth_message_names_the_tick_box_rather_than_the_scope() {
        assert!(!REAUTH_MESSAGE.contains("drive.file"));
        assert!(REAUTH_MESSAGE.contains("specific Google Drive files"));
    }


    #[test]
    fn the_suite_shares_one_visible_folder_and_mail_lives_under_it() {
        assert_eq!(FOLDER_NAME, "margin");
        assert_eq!(MAIL_FOLDER, "mail");
        assert_ne!(FOLDER_NAME, "appDataFolder");
        assert_eq!(SCOPE, "https://www.googleapis.com/auth/drive.file");
    }

}
