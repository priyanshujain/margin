use crate::dto::GrammarIssue;

#[tauri::command]
pub fn grammar_available() -> Result<bool, String> {
    Ok(margin_proof::grammar::available())
}

#[tauri::command(async)]
pub fn grammar_check(text: String) -> Result<Vec<GrammarIssue>, String> {
    margin_proof::grammar::check(&text)
}
