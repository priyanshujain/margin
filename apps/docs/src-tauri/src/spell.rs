use crate::dto::SpellIssue;

#[tauri::command(async)]
pub fn spell_check(text: String) -> Result<Vec<SpellIssue>, String> {
    Ok(margin_proof::spell::check(&text))
}

#[tauri::command(async)]
pub fn spell_learn(word: String) -> Result<(), String> {
    margin_proof::spell::learn(&word);
    Ok(())
}

#[tauri::command(async)]
pub fn spell_unlearn(word: String) -> Result<(), String> {
    margin_proof::spell::unlearn(&word);
    Ok(())
}

#[tauri::command]
pub fn spell_available() -> Result<bool, String> {
    Ok(margin_proof::spell::available())
}
