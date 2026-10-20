use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{CryptError, Phrase};

const M_COST_KIB: u32 = 64 * 1024;
const T_COST: u32 = 3;
const LANES: u32 = 1;

const ROOT_SALT: &[u8] = b"margin root key v1";
const MAIL_BACKUP_SALT: &[u8] = b"margin-mail backup phrase v1";
const APP_INFO: &str = "margin app key v1:";

#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct Key([u8; 32]);

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key(not shown)")
    }
}

impl Key {
    pub fn from_bytes(bytes: [u8; 32]) -> Key {
        Key(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RootKey(Key);

impl RootKey {
    pub fn from_phrase(phrase: &Phrase) -> Result<RootKey, CryptError> {
        argon2id(phrase, ROOT_SALT).map(RootKey)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> RootKey {
        RootKey(Key(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    pub fn app_key(&self, app: &str) -> Key {
        let mut out = [0u8; 32];
        Hkdf::<Sha256>::new(None, self.0.as_bytes())
            .expand(format!("{APP_INFO}{app}").as_bytes(), &mut out)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        Key(out)
    }
}

pub fn mail_backup_key(phrase: &Phrase) -> Result<Key, CryptError> {
    argon2id(phrase, MAIL_BACKUP_SALT)
}

fn argon2id(phrase: &Phrase, salt: &[u8]) -> Result<Key, CryptError> {
    let params = Params::new(M_COST_KIB, T_COST, LANES, Some(32))
        .map_err(|e| CryptError::Kdf(e.to_string()))?;
    let mut out = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(phrase.as_str().as_bytes(), salt, &mut out)
        .map_err(|e| CryptError::Kdf(e.to_string()))?;
    Ok(Key(out))
}
