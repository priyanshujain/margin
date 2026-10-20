use bip39::Mnemonic;
use zeroize::Zeroizing;

use crate::CryptError;

pub const WORDS: usize = 24;

#[derive(Clone, PartialEq, Eq)]
pub struct Phrase(Zeroizing<String>);

impl std::fmt::Debug for Phrase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Phrase(not shown)")
    }
}

impl Phrase {
    pub fn generate() -> Result<Phrase, CryptError> {
        Mnemonic::generate(WORDS)
            .map(|mnemonic| Phrase(Zeroizing::new(mnemonic.to_string())))
            .map_err(|e| CryptError::InvalidPhrase(e.to_string()))
    }

    pub fn parse(typed: &str) -> Result<Phrase, CryptError> {
        let tidied = Zeroizing::new(
            typed
                .split_whitespace()
                .map(str::to_lowercase)
                .collect::<Vec<String>>()
                .join(" "),
        );
        Mnemonic::parse(tidied.as_str())
            .map(|mnemonic| Phrase(Zeroizing::new(mnemonic.to_string())))
            .map_err(|e| CryptError::InvalidPhrase(e.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
