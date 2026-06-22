use fontdb::{Database, Family, Query};

fn system_db() -> Database {
    let mut db = Database::new();
    db.load_system_fonts();
    db
}

#[tauri::command(async)]
pub fn list_system_fonts() -> Vec<String> {
    let db = system_db();
    let mut names: Vec<String> = db
        .faces()
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        .collect();
    names.sort();
    names.dedup();
    names
}

pub fn system_font_bytes(family: &str) -> Vec<Vec<u8>> {
    let db = system_db();
    let mut out = Vec::new();
    let styles = [
        (fontdb::Weight::NORMAL, fontdb::Style::Normal),
        (fontdb::Weight::NORMAL, fontdb::Style::Italic),
        (fontdb::Weight::BOLD, fontdb::Style::Normal),
        (fontdb::Weight::BOLD, fontdb::Style::Italic),
    ];
    let mut seen = std::collections::HashSet::new();
    for (weight, style) in styles {
        let query = Query {
            families: &[Family::Name(family)],
            weight,
            style,
            ..Query::default()
        };
        if let Some(id) = db.query(&query) {
            if seen.insert(id) {
                if let Some(bytes) = db.with_face_data(id, |data, _index| data.to_vec()) {
                    out.push(bytes);
                }
            }
        }
    }
    out
}
