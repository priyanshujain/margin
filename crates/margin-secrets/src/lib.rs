use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use sha2::{Digest, Sha256};

const NONCE_LEN: usize = 24;

/// A key derived from `SHA-256(context || salt || machine id)`, with the 32 byte salt kept in
/// `dir/salt_file` and created on first use. Sealed values are base64 of `nonce || ciphertext`.
pub struct Vault {
    dir: PathBuf,
    salt_file: String,
    context: String,
    #[cfg(test)]
    machine: Option<&'static str>,
}

impl Vault {
    pub fn new(dir: impl Into<PathBuf>, salt_file: &str, context: &str) -> Vault {
        Vault {
            dir: dir.into(),
            salt_file: salt_file.to_string(),
            context: context.to_string(),
            #[cfg(test)]
            machine: None,
        }
    }

    pub fn seal(&self, plain: &[u8]) -> Result<String, String> {
        seal(&self.key()?, plain)
    }

    pub fn open(&self, sealed: &str) -> Result<Vec<u8>, String> {
        open(&self.key()?, sealed)
    }

    /// A salt that exists but will not read is refused rather than replaced, because writing over
    /// it would lose every stored secret.
    fn key(&self) -> Result<[u8; 32], String> {
        let path = self.dir.join(&self.salt_file);
        let salt = match std::fs::read(&path) {
            Ok(bytes) if bytes.len() == 32 => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => fresh_salt(&path)?,
            Ok(_) => fresh_salt(&path)?,
            Err(e) => return Err(format!("could not read {}: {e}", path.display())),
        };
        Ok(derive(&self.context, &salt, self.machine_id()))
    }

    #[cfg(not(test))]
    fn machine_id(&self) -> &str {
        machine_id()
    }

    #[cfg(test)]
    fn machine_id(&self) -> &str {
        self.machine.unwrap_or_else(machine_id)
    }
}

/// Many secrets in one file: a JSON object of id to sealed value. The layout Mail and Calendar use.
pub struct Entries {
    vault: Vault,
    file: String,
}

impl Entries {
    pub fn new(vault: Vault, file: &str) -> Entries {
        Entries {
            vault,
            file: file.to_string(),
        }
    }

    pub fn store(&self, id: &str, secret: &str) -> Result<(), String> {
        let sealed = self.vault.seal(secret.as_bytes())?;
        let mut entries = self.read()?;
        entries.insert(id.to_string(), sealed);
        self.write(&entries)
    }

    pub fn load(&self, id: &str) -> Result<Option<String>, String> {
        let entries = self.read()?;
        let sealed = match entries.get(id) {
            Some(sealed) => sealed,
            None => return Ok(None),
        };
        utf8(self.vault.open(sealed)?)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut entries = self.read()?;
        if entries.remove(id).is_none() {
            return Ok(());
        }
        self.write(&entries)
    }

    fn read(&self) -> Result<BTreeMap<String, String>, String> {
        let path = self.vault.dir.join(&self.file);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
            Err(e) => return Err(e.to_string()),
        };
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    fn write(&self, entries: &BTreeMap<String, String>) -> Result<(), String> {
        let text = serde_json::to_string(entries).map_err(|e| e.to_string())?;
        write_private(&self.vault.dir.join(&self.file), text.as_bytes())
    }
}

/// One secret in its own file, holding the sealed value alone. The layout Margin uses.
pub struct Single {
    vault: Vault,
    file: String,
}

impl Single {
    pub fn new(vault: Vault, file: &str) -> Single {
        Single {
            vault,
            file: file.to_string(),
        }
    }

    pub fn store(&self, secret: &str) -> Result<(), String> {
        let sealed = self.vault.seal(secret.as_bytes())?;
        write_private(&self.vault.dir.join(&self.file), sealed.as_bytes())
    }

    pub fn load(&self) -> Result<Option<String>, String> {
        let sealed = match std::fs::read_to_string(self.vault.dir.join(&self.file)) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        utf8(self.vault.open(sealed.trim())?)
    }

    pub fn delete(&self) -> Result<(), String> {
        match std::fs::remove_file(self.vault.dir.join(&self.file)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn utf8(plain: Vec<u8>) -> Result<Option<String>, String> {
    String::from_utf8(plain)
        .map(Some)
        .map_err(|_| "the stored token is not valid UTF-8".to_string())
}

fn derive(context: &str, salt: &[u8], machine: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(context.as_bytes());
    hasher.update(salt);
    hasher.update(machine.as_bytes());
    hasher.finalize().into()
}

fn fresh_salt(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    write_private(path, &bytes)?;
    Ok(bytes)
}

/// Binds the key to this machine so a copied home directory does not decrypt elsewhere. Empty on
/// iOS and Android, where the sandbox is the boundary and the available ids reset on reinstall.
pub fn machine_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(read_machine_id).as_str()
}

#[cfg(target_os = "linux")]
fn read_machine_id() -> String {
    for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(id) = std::fs::read_to_string(path) {
            let id = id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
    }
    String::new()
}

#[cfg(target_os = "macos")]
fn read_machine_id() -> String {
    let output = match std::process::Command::new("/usr/sbin/ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
    {
        Ok(output) if output.status.success() => output.stdout,
        _ => return String::new(),
    };
    String::from_utf8_lossy(&output)
        .lines()
        .find_map(|line| {
            let (_, rest) = line.split_once("\"IOPlatformUUID\"")?;
            let (_, rest) = rest.split_once('"')?;
            let (uuid, _) = rest.split_once('"')?;
            Some(uuid.to_string())
        })
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn read_machine_id() -> String {
    String::new()
}

fn seal(key_bytes: &[u8; 32], plain: &[u8]) -> Result<String, String> {
    let cipher = XChaCha20Poly1305::new(key_bytes.into());
    let mut nonce = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce);
    let mut sealed = cipher
        .encrypt((&nonce).into(), plain)
        .map_err(|_| "could not encrypt the token".to_string())?;
    let mut out = nonce.to_vec();
    out.append(&mut sealed);
    Ok(BASE64.encode(out))
}

fn open(key_bytes: &[u8; 32], sealed: &str) -> Result<Vec<u8>, String> {
    let raw = BASE64
        .decode(sealed)
        .map_err(|e| format!("the stored token is malformed: {e}"))?;
    if raw.len() <= NONCE_LEN {
        return Err("the stored token is truncated".to_string());
    }
    let (nonce, body) = raw.split_at(NONCE_LEN);
    let nonce: &XNonce = nonce
        .try_into()
        .map_err(|_| "the stored token is truncated".to_string())?;
    let cipher = XChaCha20Poly1305::new(key_bytes.into());
    cipher
        .decrypt(nonce, body)
        .map_err(|_| "the stored token could not be decrypted on this machine".to_string())
}

/// Atomic, and 0600 from creation so the bytes are never briefly world readable.
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp).map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_tokens_round_trip() {
        let key = [7u8; 32];
        let sealed = seal(&key, b"1//refresh-token").expect("seal");
        assert_eq!(open(&key, &sealed).expect("open"), b"1//refresh-token");
    }

    #[test]
    fn a_different_key_does_not_open_the_token() {
        let sealed = seal(&[7u8; 32], b"1//refresh-token").expect("seal");
        assert!(open(&[8u8; 32], &sealed).is_err());
    }

    #[test]
    fn each_seal_uses_a_fresh_nonce() {
        let key = [7u8; 32];
        assert_ne!(
            seal(&key, b"same").expect("seal"),
            seal(&key, b"same").expect("seal")
        );
    }

    #[test]
    fn a_truncated_blob_is_rejected_rather_than_panicking() {
        assert!(open(&[7u8; 32], &BASE64.encode([0u8; 8])).is_err());
    }

    #[test]
    fn a_tampered_blob_is_rejected_rather_than_returning_plaintext() {
        let key = [7u8; 32];
        let sealed = seal(&key, b"1//refresh-token").expect("seal");
        let mut raw = BASE64.decode(&sealed).expect("decode");
        let last = raw.len() - 1;
        raw[last] ^= 0x01;
        assert!(open(&key, &BASE64.encode(raw)).is_err());
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn a_desktop_binds_the_key_to_the_machine() {
        assert!(
            !machine_id().is_empty(),
            "no machine id, so a copied home directory would decrypt anywhere"
        );
    }

    #[test]
    fn the_machine_id_is_stable_within_a_run() {
        assert_eq!(machine_id(), machine_id());
    }

    #[test]
    fn a_stored_token_comes_back_out() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let entries = Entries::new(
            Vault::new(dir.path(), "tokens.salt", "margin-mail token store v1"),
            "tokens.enc",
        );
        entries.store("account", "1//refresh-token").expect("store");
        assert_eq!(
            entries.load("account").expect("load").as_deref(),
            Some("1//refresh-token")
        );
        assert_eq!(
            entries.load("account").expect("load").as_deref(),
            Some("1//refresh-token")
        );
        entries.delete("account").expect("delete");
        assert_eq!(entries.load("account").expect("load"), None);
    }

    const FIXTURE_MACHINE: &str = "FIXTURE-MACHINE-0000";

    struct Fixture {
        dir: tempfile::TempDir,
        salt_file: &'static str,
    }

    fn fixture(app: &str, salt_file: &'static str, blob_file: &str) -> Fixture {
        let dir = tempfile::tempdir().expect("a temp dir");
        let from = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(app);
        for name in [salt_file, blob_file] {
            std::fs::copy(from.join(name), dir.path().join(name)).expect("a fixture file");
        }
        Fixture { dir, salt_file }
    }

    impl Fixture {
        fn vault(&self, context: &str) -> Vault {
            let mut vault = Vault::new(self.dir.path(), self.salt_file, context);
            vault.machine = Some(FIXTURE_MACHINE);
            vault
        }
    }

    #[test]
    fn opens_a_mail_token_file_written_by_the_original_code() {
        let fixture = fixture("mail", "tokens.salt", "tokens.enc");
        let entries = Entries::new(fixture.vault("margin-mail token store v1"), "tokens.enc");
        assert_eq!(
            entries.load("fixture-account").expect("load").as_deref(),
            Some("1//fake-mail-refresh-token")
        );
        assert_eq!(
            entries.load("second-account").expect("load").as_deref(),
            Some("1//fake-second-token")
        );
    }

    #[test]
    fn opens_a_calendar_token_file_written_by_the_original_code() {
        let fixture = fixture("calendar", "tokens.salt", "tokens.enc");
        let entries = Entries::new(
            fixture.vault("margin-calendar token store v1"),
            "tokens.enc",
        );
        assert_eq!(
            entries.load("fixture-account").expect("load").as_deref(),
            Some("1//fake-calendar-refresh-token")
        );
    }

    #[test]
    fn opens_a_margin_drive_token_written_by_the_original_code() {
        let fixture = fixture("margin", "gdrive-token.salt", "gdrive-token.enc");
        let single = Single::new(fixture.vault("margin gdrive token v1"), "gdrive-token.enc");
        assert_eq!(
            single.load().expect("load").as_deref(),
            Some("1//fake-drive-refresh-token")
        );
    }

    #[test]
    fn a_rewrite_keeps_the_existing_salt_and_entries() {
        let fixture = fixture("mail", "tokens.salt", "tokens.enc");
        let salt = std::fs::read(fixture.dir.path().join("tokens.salt")).expect("salt");
        let entries = Entries::new(fixture.vault("margin-mail token store v1"), "tokens.enc");
        entries.store("third-account", "1//new").expect("store");
        assert_eq!(
            std::fs::read(fixture.dir.path().join("tokens.salt")).expect("salt"),
            salt
        );
        assert_eq!(
            entries.load("fixture-account").expect("load").as_deref(),
            Some("1//fake-mail-refresh-token")
        );
    }

    #[test]
    fn the_wrong_context_or_machine_does_not_open_a_fixture() {
        let fixture = fixture("mail", "tokens.salt", "tokens.enc");
        let other_app = Entries::new(
            fixture.vault("margin-calendar token store v1"),
            "tokens.enc",
        );
        assert!(other_app.load("fixture-account").is_err());
        let mut vault = fixture.vault("margin-mail token store v1");
        vault.machine = Some("ANOTHER-MACHINE");
        assert!(
            Entries::new(vault, "tokens.enc")
                .load("fixture-account")
                .is_err()
        );
    }
}
