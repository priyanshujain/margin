// Spelling is the system's where there is one. On macOS that is NSSpellChecker, whose learned words
// are shared with every other app. Elsewhere it is Hunspell over a dictionary the app hands to
// `use_dictionary`; until it does, every call is an empty answer, not an error.

use crate::SpellIssue;

#[cfg(target_os = "macos")]
use crate::macspell as checker;
#[cfg(not(target_os = "macos"))]
use self::fallback as checker;

#[cfg(not(target_os = "macos"))]
mod fallback {
    use crate::SpellIssue;

    pub fn check(text: &str) -> Vec<SpellIssue> {
        super::hunspell::with(|checker| checker.check(text)).unwrap_or_default()
    }

    pub fn learn(word: &str) {
        super::hunspell::with(|checker| checker.learn(word));
    }

    pub fn unlearn(word: &str) {
        super::hunspell::with(|checker| checker.unlearn(word));
    }
}

/// Loads the Hunspell dictionary the non-macOS checker uses. Ignored on macOS, where the system
/// checker is always there. Returns an error, and leaves spelling unavailable, for a bad dictionary.
pub fn use_dictionary(aff: &str, dic: &str) -> Result<(), String> {
    hunspell::install(aff, dic)
}

pub fn check(text: &str) -> Vec<SpellIssue> {
    checker::check(text)
}

pub fn learn(word: &str) {
    let word = word.trim();
    if !word.is_empty() {
        checker::learn(word);
    }
}

pub fn unlearn(word: &str) {
    let word = word.trim();
    if !word.is_empty() {
        checker::unlearn(word);
    }
}

pub fn available() -> bool {
    cfg!(target_os = "macos") || hunspell::installed()
}

pub mod hunspell {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};

    use crate::SpellIssue;

    const MAX_SUGGESTIONS: usize = 5;

    static CHECKER: OnceLock<Mutex<Checker>> = OnceLock::new();

    pub(crate) fn install(aff: &str, dic: &str) -> Result<(), String> {
        let checker = Checker::new(aff, dic)?;
        match CHECKER.get() {
            Some(slot) => *slot.lock().map_err(|e| e.to_string())? = checker,
            None => {
                let _ = CHECKER.set(Mutex::new(checker));
            }
        }
        Ok(())
    }

    pub(crate) fn installed() -> bool {
        CHECKER.get().is_some()
    }

    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub(crate) fn with<T>(f: impl FnOnce(&mut Checker) -> T) -> Option<T> {
        let mut checker = CHECKER.get()?.lock().ok()?;
        Some(f(&mut checker))
    }

    /// A Hunspell dictionary plus the words learned this run. Offsets are in characters.
    pub struct Checker {
        dictionary: spellbook::Dictionary,
        learned: HashSet<String>,
    }

    impl Checker {
        pub fn new(aff: &str, dic: &str) -> Result<Checker, String> {
            let dictionary =
                spellbook::Dictionary::new(aff, dic).map_err(|e| format!("dictionary: {e:?}"))?;
            Ok(Checker {
                dictionary,
                learned: HashSet::new(),
            })
        }

        pub fn learn(&mut self, word: &str) {
            self.learned.insert(word.to_lowercase());
        }

        pub fn unlearn(&mut self, word: &str) {
            self.learned.remove(&word.to_lowercase());
        }

        pub fn check(&self, text: &str) -> Vec<SpellIssue> {
            let chars: Vec<char> = text.chars().collect();
            let mut issues = Vec::new();
            let mut i = 0;
            while i < chars.len() {
                if !chars[i].is_alphabetic() {
                    i += 1;
                    continue;
                }
                let start = i;
                i += 1;
                while i < chars.len() {
                    let joins = is_apostrophe(chars[i])
                        && chars.get(i + 1).is_some_and(|c| c.is_alphabetic());
                    if chars[i].is_alphabetic() || joins {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let word: String = chars[start..i].iter().collect();
                if let Some(suggestions) = self.misspelt(&word) {
                    issues.push(SpellIssue {
                        start,
                        end: i,
                        word,
                        suggestions,
                    });
                }
            }
            issues
        }

        fn misspelt(&self, word: &str) -> Option<Vec<String>> {
            let irregular_case = word.chars().skip(1).any(char::is_uppercase);
            if irregular_case
                || self.learned.contains(&word.to_lowercase())
                || self.dictionary.check(word)
            {
                return None;
            }
            let mut suggestions = Vec::new();
            self.dictionary.suggest(word, &mut suggestions);
            let lower = word.to_lowercase();
            if suggestions.iter().any(|s| s.to_lowercase() == lower) {
                return None;
            }
            suggestions.truncate(MAX_SUGGESTIONS);
            Some(suggestions)
        }
    }

    fn is_apostrophe(c: char) -> bool {
        c == '\'' || c == '\u{2019}'
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const AFF: &str = "SET UTF-8\nTRY esianrtolcdugmphbyfvkwz'\n\nSFX S Y 1\nSFX S 0 s .\n";
        const DIC: &str = "4\nhello\nworld/S\nit's\nquick\n";

        #[test]
        fn flags_unknown_words_with_character_offsets() {
            let checker = Checker::new(AFF, DIC).unwrap();
            let issues = checker.check("héllo worlds, quikc it's");
            let words: Vec<_> = issues.iter().map(|i| (i.word.as_str(), i.start, i.end)).collect();
            assert_eq!(words, vec![("héllo", 0, 5), ("quikc", 14, 19)]);
        }

        #[test]
        fn learned_words_and_irregular_case_pass() {
            let mut checker = Checker::new(AFF, DIC).unwrap();
            checker.learn("Quikc");
            assert!(checker.check("quikc iPhone").is_empty());
            checker.unlearn("quikc");
            assert_eq!(checker.check("quikc").len(), 1);
        }
    }
}
