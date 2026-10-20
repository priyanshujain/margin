use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptError {
    InvalidPhrase(String),
    NotSealed,
    Truncated,
    Decrypt,
    Encrypt,
    Kdf(String),
}

impl fmt::Display for CryptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptError::InvalidPhrase(detail) => write!(f, "that is not a recovery phrase: {detail}"),
            CryptError::NotSealed => f.write_str("the object is not in a recognised sealed format"),
            CryptError::Truncated => f.write_str("the sealed object is truncated"),
            CryptError::Decrypt => f.write_str(
                "the object could not be decrypted: the key is wrong, or the object or its name changed since it was sealed",
            ),
            CryptError::Encrypt => f.write_str("the object could not be encrypted"),
            CryptError::Kdf(detail) => write!(f, "key derivation failed: {detail}"),
        }
    }
}

impl std::error::Error for CryptError {}
