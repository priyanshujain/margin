use std::path::{Path, PathBuf};

use margin_crypt::{Phrase, RootKey};
use margin_secrets::{Entries, Vault};

use crate::storage::Storage;

const STORAGE_FILE: &str = "sync-storage.json";
const SECRETS_FILE: &str = "sync-secrets.enc";
const SALT_FILE: &str = "sync-secrets.salt";
const CONTEXT: &str = "margin sync secrets v1";

const PHRASE_ID: &str = "recovery phrase";
const ROOT_KEY_ID: &str = "root key";
const STORAGE_SECRET_ID: &str = "storage secret";

/// The suite-wide sync settings for this machine: which store, and the recovery phrase. Lives in
/// the shared directory so every Margin app sees the same answer. The phrase, its root key and
/// the store's secret are sealed with margin-secrets; the rest is plain JSON.
pub struct Suite {
    dir: PathBuf,
}

impl Suite {
    pub fn new(dir: impl Into<PathBuf>) -> Suite {
        Suite { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn secrets(&self) -> Result<Entries, String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        Ok(Entries::new(
            Vault::new(&self.dir, SALT_FILE, CONTEXT),
            SECRETS_FILE,
        ))
    }

    pub fn storage(&self) -> Result<Option<Storage>, String> {
        let path = self.dir.join(STORAGE_FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("could not read {}: {e}", path.display())),
        };
        let mut storage: Storage = serde_json::from_str(&text)
            .map_err(|e| format!("{} is not readable: {e}", path.display()))?;
        if storage.secret().is_some()
            && let Some(secret) = self.secrets()?.load(STORAGE_SECRET_ID)?
        {
            storage.set_secret(secret);
        }
        Ok(Some(storage))
    }

    pub fn set_storage(&self, storage: &Storage) -> Result<(), String> {
        let secrets = self.secrets()?;
        match storage.secret() {
            Some(secret) => secrets.store(STORAGE_SECRET_ID, secret)?,
            None => secrets.delete(STORAGE_SECRET_ID)?,
        }
        let text = serde_json::to_string_pretty(storage).map_err(|e| e.to_string())?;
        write_atomic(&self.dir.join(STORAGE_FILE), text.as_bytes())
    }

    pub fn clear_storage(&self) -> Result<(), String> {
        self.secrets()?.delete(STORAGE_SECRET_ID)?;
        match std::fs::remove_file(self.dir.join(STORAGE_FILE)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn phrase(&self) -> Result<Option<Phrase>, String> {
        match self.secrets()?.load(PHRASE_ID)? {
            Some(words) => Phrase::parse(&words).map(Some).map_err(|e| e.to_string()),
            None => Ok(None),
        }
    }

    /// Stores the phrase and the root key derived from it, so apps do not pay for Argon2 on
    /// every pass.
    pub fn set_phrase(&self, phrase: &Phrase) -> Result<RootKey, String> {
        let root = RootKey::from_phrase(phrase).map_err(|e| e.to_string())?;
        self.set_phrase_with_key(phrase, &root)?;
        Ok(root)
    }

    pub fn set_phrase_with_key(&self, phrase: &Phrase, root: &RootKey) -> Result<(), String> {
        let secrets = self.secrets()?;
        secrets.store(ROOT_KEY_ID, &hex(root.as_bytes()))?;
        secrets.store(PHRASE_ID, phrase.as_str())
    }

    pub fn root_key(&self) -> Result<Option<RootKey>, String> {
        let Some(held) = self.secrets()?.load(ROOT_KEY_ID)? else {
            return Ok(None);
        };
        let bytes = unhex(&held).ok_or("the stored sync key is malformed")?;
        Ok(Some(RootKey::from_bytes(bytes)))
    }

    pub fn forget_phrase(&self) -> Result<(), String> {
        let secrets = self.secrets()?;
        secrets.delete(PHRASE_ID)?;
        secrets.delete(ROOT_KEY_ID)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (at, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(at * 2..at * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_round_trips_and_its_secret_never_reaches_the_plain_file() {
        let dir = tempfile::tempdir().unwrap();
        let suite = Suite::new(dir.path());
        assert_eq!(suite.storage().unwrap(), None);
        let storage = Storage::S3 {
            endpoint: "https://example.r2.cloudflarestorage.com".into(),
            region: "auto".into(),
            bucket: "margin".into(),
            access_key: "AKID".into(),
            secret: "very-secret".into(),
            prefix: None,
            allow_http: false,
        };
        suite.set_storage(&storage).unwrap();
        let plain = std::fs::read_to_string(dir.path().join(STORAGE_FILE)).unwrap();
        assert!(!plain.contains("very-secret"));
        assert_eq!(suite.storage().unwrap(), Some(storage));

        suite
            .set_storage(&Storage::Folder {
                path: "/tmp/x".into(),
            })
            .unwrap();
        assert_eq!(
            suite.secrets().unwrap().load(STORAGE_SECRET_ID).unwrap(),
            None
        );
        suite.clear_storage().unwrap();
        assert_eq!(suite.storage().unwrap(), None);
    }

    #[test]
    fn a_second_app_on_the_machine_reads_the_same_phrase_and_key() {
        let dir = tempfile::tempdir().unwrap();
        let phrase = Phrase::generate().unwrap();
        let root = RootKey::from_bytes([7; 32]);
        Suite::new(dir.path())
            .set_phrase_with_key(&phrase, &root)
            .unwrap();

        let other = Suite::new(dir.path());
        assert_eq!(other.phrase().unwrap(), Some(phrase));
        assert_eq!(other.root_key().unwrap(), Some(root));
        let sealed = std::fs::read_to_string(dir.path().join(SECRETS_FILE)).unwrap();
        assert!(!sealed.contains(other.phrase().unwrap().unwrap().as_str()));

        other.forget_phrase().unwrap();
        assert_eq!(Suite::new(dir.path()).phrase().unwrap(), None);
        assert_eq!(Suite::new(dir.path()).root_key().unwrap(), None);
    }
}
