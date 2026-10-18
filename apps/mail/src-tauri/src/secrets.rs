// Refresh tokens, IMAP passwords and the backup key: sealed entries in `tokens.enc`, keyed by a salt
// in `tokens.salt`, in the app data directory. The format and the file names are margin-secrets'
// `Entries` layout and must not change, or every connected account has to sign in again.

use std::path::PathBuf;
use std::sync::OnceLock;

use margin_secrets::{Entries, Vault};

pub const SERVICE: &str = "studio.margin.mail";

const BLOB_NAME: &str = "tokens.enc";
const SALT_NAME: &str = "tokens.salt";
const KEY_CONTEXT: &str = "margin-mail token store v1";

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Called from `auth::init_sessions` during setup, before any command can run.
pub fn init(dir: PathBuf) {
    let _ = DATA_DIR.set(dir);
}

pub fn store(id: &str, secret: &str) -> Result<(), String> {
    entries()?.store(id, secret)
}

pub fn load(id: &str) -> Result<Option<String>, String> {
    entries()?.load(id)
}

pub fn delete(id: &str) -> Result<(), String> {
    entries()?.delete(id)
}

fn entries() -> Result<Entries, String> {
    Ok(Entries::new(
        Vault::new(data_dir()?, SALT_NAME, KEY_CONTEXT),
        BLOB_NAME,
    ))
}

/// Only reachable without `init` before setup has run, which on mobile is never.
fn data_dir() -> Result<PathBuf, String> {
    if let Some(dir) = DATA_DIR.get() {
        return Ok(dir.clone());
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or("no app data directory is known")?;
    let dir = base.join(SERVICE);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The only test that touches `DATA_DIR`, which is set once per process and never reset.
    #[test]
    fn a_stored_token_comes_back_out() {
        let dir = tempfile::tempdir().expect("a temp dir");
        init(dir.path().to_path_buf());

        store("account", "1//refresh-token").expect("store");
        assert_eq!(
            load("account").expect("load").as_deref(),
            Some("1//refresh-token")
        );
        assert_eq!(
            load("account").expect("load").as_deref(),
            Some("1//refresh-token")
        );

        delete("account").expect("delete");
        assert_eq!(load("account").expect("load"), None);
    }
}
