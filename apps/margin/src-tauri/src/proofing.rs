use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use margin_proof::{GrammarIssue, SpellIssue};
use tauri::Manager;
use tauri::path::BaseDirectory;

pub const DICTIONARY_NAME: &str = "custom-dictionary.txt";

/// Words this app accepts on top of the system checker: the user's own list, which is backed up
/// with the books, and the bundled technical vocabulary. Lower cased.
#[derive(Default)]
pub struct CustomWords(Mutex<Option<HashSet<String>>>);

fn dict_file(app: &tauri::AppHandle, name: &str) -> Result<String, String> {
    let rel = format!("resources/dictionaries/en/{name}");
    if let Ok(path) = app.path().resolve(&rel, BaseDirectory::Resource)
        && let Ok(contents) = fs::read_to_string(&path)
    {
        return Ok(contents);
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&rel);
    fs::read_to_string(&dev).map_err(|e| format!("{name}: {e}"))
}

pub fn custom_dictionary_path(dir: &Path) -> PathBuf {
    dir.join(DICTIONARY_NAME)
}

pub fn read_words(contents: &str, set: &mut HashSet<String>) {
    for line in contents.lines() {
        let word = line.trim();
        if !word.is_empty() && !word.starts_with('#') {
            set.insert(word.to_lowercase());
        }
    }
}

/// Appends one word to the custom dictionary in `dir`, which is the format the app has always kept.
pub fn append_word(dir: &Path, word: &str) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(custom_dictionary_path(dir))
        .map_err(|e| e.to_string())?;
    writeln!(file, "{word}").map_err(|e| e.to_string())
}

pub fn without_custom(issues: Vec<SpellIssue>, custom: &HashSet<String>) -> Vec<SpellIssue> {
    issues
        .into_iter()
        .filter(|issue| !custom.contains(&issue.word.to_lowercase()))
        .collect()
}

fn load_custom(app: &tauri::AppHandle) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Ok(dir) = margin_shell::dirs::app_data_dir(app)
        && let Ok(contents) = fs::read_to_string(custom_dictionary_path(&dir))
    {
        read_words(&contents, &mut set);
    }
    if let Ok(contents) = dict_file(app, "tech.txt") {
        read_words(&contents, &mut set);
    }
    set
}

/// Hands the bundled Hunspell dictionary to margin-proof where there is no system checker.
pub fn install(app: &tauri::AppHandle) {
    if cfg!(target_os = "macos") {
        return;
    }
    match (dict_file(app, "index.aff"), dict_file(app, "index.dic")) {
        (Ok(aff), Ok(dic)) => {
            if let Err(e) = margin_proof::spell::use_dictionary(&aff, &dic) {
                margin_log::note("spelling", &e);
            }
        }
        (Err(e), _) | (_, Err(e)) => margin_log::note("spelling", &e),
    }
}

fn with_custom<T>(
    app: &tauri::AppHandle,
    f: impl FnOnce(&mut HashSet<String>) -> T,
) -> Result<T, String> {
    let state = app.state::<CustomWords>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let words = guard.get_or_insert_with(|| load_custom(app));
    Ok(f(words))
}

#[tauri::command(async)]
pub fn spell_check(app: tauri::AppHandle, text: String) -> Result<Vec<SpellIssue>, String> {
    let issues = margin_proof::spell::check(&text);
    with_custom(&app, |custom| without_custom(issues, custom))
}

#[tauri::command(async)]
pub fn spell_learn(app: tauri::AppHandle, word: String) -> Result<(), String> {
    let word = word.trim().to_string();
    if word.is_empty() {
        return Ok(());
    }
    append_word(&margin_shell::dirs::app_data_dir(&app)?, &word)?;
    with_custom(&app, |custom| {
        custom.insert(word.to_lowercase());
    })
}

#[tauri::command(async)]
pub fn spell_unlearn(app: tauri::AppHandle, word: String) -> Result<(), String> {
    with_custom(&app, |custom| {
        custom.remove(&word.trim().to_lowercase());
    })
}

#[tauri::command]
pub fn spell_available() -> bool {
    margin_proof::spell::available()
}

#[tauri::command]
pub fn grammar_available() -> bool {
    margin_proof::grammar::available()
}

#[tauri::command(async)]
pub fn grammar_check(text: String) -> Result<Vec<GrammarIssue>, String> {
    margin_proof::grammar::check(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(word: &str) -> SpellIssue {
        SpellIssue {
            start: 0,
            end: word.chars().count(),
            word: word.to_string(),
            suggestions: Vec::new(),
        }
    }

    #[test]
    fn learned_words_append_to_the_custom_dictionary_and_are_not_flagged() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(custom_dictionary_path(dir.path()), "# mine\nTauri\n").unwrap();
        append_word(dir.path(), "Zustand").unwrap();
        let text = fs::read_to_string(custom_dictionary_path(dir.path())).unwrap();
        assert_eq!(text, "# mine\nTauri\nZustand\n");

        let mut words = HashSet::new();
        read_words(&text, &mut words);
        let left = without_custom(vec![issue("tauri"), issue("ZUSTAND"), issue("teh")], &words);
        assert_eq!(
            left.iter().map(|i| i.word.as_str()).collect::<Vec<_>>(),
            ["teh"]
        );
    }
}
