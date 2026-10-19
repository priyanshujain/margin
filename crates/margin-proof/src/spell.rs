// Spelling is the system's. On macOS that is NSSpellChecker, whose learned words are shared with
// every other app; elsewhere there is no checker and every call is an empty answer, not an error.

use crate::SpellIssue;

#[cfg(not(target_os = "macos"))]
use self::no_checker as checker;
#[cfg(target_os = "macos")]
use crate::macspell as checker;

#[cfg(not(target_os = "macos"))]
mod no_checker {
    use crate::SpellIssue;

    pub fn check(_text: &str) -> Vec<SpellIssue> {
        Vec::new()
    }

    pub fn learn(_word: &str) {}

    pub fn unlearn(_word: &str) {}
}

/// Misspellings in one run of text, as half-open character offsets from the start of that run.
pub fn check(text: &str) -> Vec<SpellIssue> {
    checker::check(text)
}

/// Teaches a word to the system dictionary. A blank word is ignored.
pub fn learn(word: &str) {
    let word = word.trim();
    if !word.is_empty() {
        checker::learn(word);
    }
}

/// Undoes `learn`. A blank word is ignored.
pub fn unlearn(word: &str) {
    let word = word.trim();
    if !word.is_empty() {
        checker::unlearn(word);
    }
}

pub fn available() -> bool {
    cfg!(target_os = "macos")
}
