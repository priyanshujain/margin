use std::collections::HashSet;

use objc2::rc::autoreleasepool;
use objc2_app_kit::NSSpellChecker;
use objc2_foundation::{NSRange, NSString, NSTextCheckingType};

pub struct MacIssue {
    pub start: usize,
    pub end: usize,
    pub word: String,
    pub suggestions: Vec<String>,
}

fn utf16_to_codepoint(text: &str, utf16_len: usize) -> Vec<usize> {
    let mut map = Vec::with_capacity(utf16_len + 1);
    let mut cp = 0;
    for ch in text.chars() {
        for _ in 0..ch.len_utf16() {
            map.push(cp);
        }
        cp += 1;
    }
    map.push(cp);
    map
}

pub fn check(text: &str, custom: &HashSet<String>) -> Vec<MacIssue> {
    autoreleasepool(|_| {
        let checker = NSSpellChecker::sharedSpellChecker();
        let ns = NSString::from_str(text);
        let len = ns.length();
        let results = unsafe {
            checker.checkString_range_types_options_inSpellDocumentWithTag_orthography_wordCount(
                &ns,
                NSRange { location: 0, length: len },
                (NSTextCheckingType::Spelling | NSTextCheckingType::Link).bits(),
                None,
                0,
                None,
                std::ptr::null_mut(),
            )
        };

        let map = utf16_to_codepoint(text, len);
        let chars: Vec<char> = text.chars().collect();
        let mut issues = Vec::new();
        for result in results.iter() {
            if result.resultType() != NSTextCheckingType::Spelling {
                continue;
            }
            let range = result.range();
            let start = map[range.location.min(len)];
            let end = map[(range.location + range.length).min(len)];
            let word: String = chars[start..end].iter().collect();
            if custom.contains(&word.to_lowercase()) {
                continue;
            }
            let mut suggestions = Vec::new();
            if let Some(guesses) =
                checker.guessesForWordRange_inString_language_inSpellDocumentWithTag(range, &ns, None, 0)
            {
                for guess in guesses.iter() {
                    suggestions.push(guess.to_string());
                    if suggestions.len() >= 5 {
                        break;
                    }
                }
            }
            issues.push(MacIssue { start, end, word, suggestions });
        }
        issues
    })
}
