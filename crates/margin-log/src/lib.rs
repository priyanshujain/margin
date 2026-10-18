use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Past this the file is cut back to its newer half, from a line boundary, before the next line
/// goes on.
pub const CAP_BYTES: u64 = 256 * 1024;

/// A line longer than this is cut, because one stack trace can be the whole cap.
const LINE_CAP: usize = 2_000;

static PATH: OnceLock<PathBuf> = OnceLock::new();
static WRITING: Mutex<()> = Mutex::new(());

/// Names the file. Called once by whoever is handed the app data directory first; before that, and
/// in tests, a note goes to stderr alone.
pub fn init(dir: &Path, file: &str) {
    let _ = PATH.set(dir.join(file));
}

pub fn path() -> Option<&'static Path> {
    PATH.get().map(PathBuf::as_path)
}

/// One line, on stderr and in the file. `who` is an account id or an area such as `ui`.
pub fn note(who: &str, line: &str) {
    let line = trim(line);
    eprintln!("[{who}] {line}");
    let Some(path) = PATH.get() else { return };
    let stamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let entry = format!("{stamp} {who} {line}\n");
    let _held = WRITING.lock();
    let _ = append(path, &entry, CAP_BYTES);
}

fn trim(line: &str) -> String {
    let flat = line.replace(['\n', '\r'], " ");
    if flat.chars().count() <= LINE_CAP {
        return flat;
    }
    let mut cut: String = flat.chars().take(LINE_CAP).collect();
    cut.push_str(" [cut]");
    cut
}

fn append(path: &Path, entry: &str, cap: u64) -> std::io::Result<()> {
    use std::io::Write;
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size + entry.len() as u64 > cap {
        let held = std::fs::read(path).unwrap_or_default();
        let from = held.len().saturating_sub((cap / 2) as usize);
        let cut = held[from..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|at| from + at + 1)
            .unwrap_or(from);
        let mut kept = held[cut..].to_vec();
        kept.extend_from_slice(entry.as_bytes());
        return std::fs::write(path, kept);
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(entry.as_bytes())
}

/// The frontend's way in: failed commands and uncaught webview errors.
#[cfg(feature = "tauri")]
pub mod commands {
    #[tauri::command]
    pub fn log_note(who: String, line: String) {
        super::note(&who, &line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("margin-log-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&dir);
        dir
    }

    #[test]
    fn lines_go_on_the_end_and_the_file_never_outgrows_its_cap() {
        let path = scratch("cap");
        let line = "2026-09-05T10:00:00Z acct other: Gmail label change failed (400): no\n";
        for _ in 0..200 {
            append(&path, line, 2_048).expect("append");
        }
        let held = std::fs::read_to_string(&path).expect("the file");
        assert!(held.len() as u64 <= 2_048, "was {}", held.len());
        assert!(held.ends_with(line));
        assert!(held.starts_with("2026-"), "{:?}", &held[..40]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_note_before_init_is_not_an_error() {
        note("acct", "network: the train went into a tunnel");
    }

    #[test]
    fn a_line_is_one_line_and_a_long_one_is_cut() {
        assert_eq!(trim("a\nb\r\nc"), "a b  c");
        let long = "x".repeat(LINE_CAP + 50);
        let cut = trim(&long);
        assert!(cut.ends_with(" [cut]"));
        assert_eq!(cut.chars().count(), LINE_CAP + " [cut]".len());
    }
}
