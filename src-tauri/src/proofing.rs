use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use harper_core::linting::{LintGroup, Linter, Suggestion};
use harper_core::spell::FstDictionary;
use harper_core::{Dialect, Document};
use serde::Serialize;
use tauri::path::BaseDirectory;
use tauri::Manager;

#[derive(Serialize)]
pub struct Issue {
    start: usize,
    end: usize,
    kind: String,
    category: String,
    message: String,
    suggestions: Vec<String>,
}

struct Harper {
    linter: LintGroup,
    dict: Arc<FstDictionary>,
}

pub struct Engine {
    speller: Option<spellbook::Dictionary>,
    harper: Option<Harper>,
    custom: HashSet<String>,
}

pub type ProofState = Mutex<Option<Engine>>;

pub fn new_state() -> ProofState {
    Mutex::new(None)
}

fn dict_file(app: &tauri::AppHandle, name: &str) -> Result<String, String> {
    let rel = format!("resources/dictionaries/en/{name}");
    if let Ok(path) = app.path().resolve(&rel, BaseDirectory::Resource) {
        if let Ok(contents) = fs::read_to_string(&path) {
            return Ok(contents);
        }
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&rel);
    fs::read_to_string(&dev).map_err(|e| format!("{name}: {e}"))
}

fn custom_dict_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("custom-dictionary.txt"))
}

fn read_words(contents: &str, set: &mut HashSet<String>) {
    for line in contents.lines() {
        let word = line.trim();
        if !word.is_empty() && !word.starts_with('#') {
            set.insert(word.to_string());
        }
    }
}

fn load_custom(app: &tauri::AppHandle) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Ok(path) = custom_dict_path(app) {
        if let Ok(contents) = fs::read_to_string(&path) {
            read_words(&contents, &mut set);
        }
    }
    #[cfg(target_os = "macos")]
    if let Ok(home) = app.path().home_dir() {
        let local = home.join("Library/Spelling/LocalDictionary");
        if let Ok(contents) = fs::read_to_string(&local) {
            read_words(&contents, &mut set);
        }
    }
    set
}

fn build_speller(app: &tauri::AppHandle) -> Result<spellbook::Dictionary, String> {
    let aff = dict_file(app, "index.aff")?;
    let dic = dict_file(app, "index.dic")?;
    spellbook::Dictionary::new(&aff, &dic).map_err(|e| format!("dictionary: {e:?}"))
}

fn build_harper() -> Harper {
    let dict = FstDictionary::curated();
    let mut linter = LintGroup::new_curated(dict.clone(), Dialect::American);
    linter.config.set_rule_enabled("SpellCheck", false);
    Harper { linter, dict }
}

fn is_word_char(c: char) -> bool {
    c.is_alphabetic()
}

fn is_apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

fn collect_spelling(
    speller: &spellbook::Dictionary,
    custom: &HashSet<String>,
    chars: &[char],
    issues: &mut Vec<Issue>,
) {
    let n = chars.len();
    let mut i = 0;
    while i < n {
        if !is_word_char(chars[i]) {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        while i < n {
            if is_word_char(chars[i]) {
                i += 1;
            } else if is_apostrophe(chars[i]) && i + 1 < n && is_word_char(chars[i + 1]) {
                i += 1;
            } else {
                break;
            }
        }
        let end = i;
        let word: String = chars[start..end].iter().collect();
        if speller.check(&word) || custom.contains(&word) {
            continue;
        }
        let mut suggestions = Vec::new();
        speller.suggest(&word, &mut suggestions);
        suggestions.truncate(5);
        issues.push(Issue {
            start,
            end,
            kind: "spelling".into(),
            category: "Spelling".into(),
            message: format!("“{word}” may be misspelled"),
            suggestions,
        });
    }
}

fn collect_grammar(harper: &mut Harper, text: &str, chars: &[char], issues: &mut Vec<Issue>) {
    let doc = Document::new_plain_english(text, harper.dict.as_ref());
    for lint in harper.linter.lint(&doc) {
        let start = lint.span.start.min(chars.len());
        let end = lint.span.end.min(chars.len());
        let existing: String = chars[start..end].iter().collect();
        let mut suggestions = Vec::new();
        for suggestion in &lint.suggestions {
            match suggestion {
                Suggestion::ReplaceWith(replacement) => suggestions.push(replacement.iter().collect()),
                Suggestion::InsertAfter(insertion) => {
                    suggestions.push(format!("{existing}{}", insertion.iter().collect::<String>()))
                }
                Suggestion::Remove => suggestions.push(String::new()),
            }
        }
        suggestions.truncate(5);
        issues.push(Issue {
            start,
            end,
            kind: "grammar".into(),
            category: format!("{:?}", lint.lint_kind),
            message: lint.message,
            suggestions,
        });
    }
}

#[tauri::command]
pub async fn proof_text(
    app: tauri::AppHandle,
    state: tauri::State<'_, ProofState>,
    text: String,
    spelling: bool,
    grammar: bool,
) -> Result<Vec<Issue>, String> {
    let mut guard = state.inner().lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(Engine {
            speller: None,
            harper: None,
            custom: load_custom(&app),
        });
    }
    let engine = guard.as_mut().unwrap();

    let chars: Vec<char> = text.chars().collect();
    let mut issues = Vec::new();
    if spelling {
        if engine.speller.is_none() {
            engine.speller = Some(build_speller(&app)?);
        }
        collect_spelling(engine.speller.as_ref().unwrap(), &engine.custom, &chars, &mut issues);
    }
    if grammar {
        if engine.harper.is_none() {
            engine.harper = Some(build_harper());
        }
        collect_grammar(engine.harper.as_mut().unwrap(), &text, &chars, &mut issues);
    }
    Ok(issues)
}

#[tauri::command]
pub fn remember_word(
    app: tauri::AppHandle,
    state: tauri::State<'_, ProofState>,
    word: String,
) -> Result<(), String> {
    let word = word.trim().to_string();
    if word.is_empty() {
        return Ok(());
    }
    let path = custom_dict_path(&app)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{word}").map_err(|e| e.to_string())?;

    let mut guard = state.inner().lock().map_err(|e| e.to_string())?;
    if let Some(engine) = guard.as_mut() {
        engine.custom.insert(word);
    }
    Ok(())
}
