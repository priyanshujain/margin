use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use chrono::{Local, NaiveDate};

/// A day's file is cut back to its newer half, from a line boundary, past this size.
pub const CAP_BYTES: u64 = 5 * 1024 * 1024;

/// Day files older than this are deleted when the log starts and when the day changes.
pub const KEEP_DAYS: i64 = 7;

/// A line longer than this is cut, because one stack trace can be the whole cap.
const LINE_CAP: usize = 2_000;

struct Target {
    dir: PathBuf,
    stem: String,
}

static TARGET: OnceLock<Target> = OnceLock::new();
static WRITING: Mutex<Option<NaiveDate>> = Mutex::new(None);

/// Points the log at `<dir>/logs/<stem>-YYYY-MM-DD.log`, where `file` is `<stem>.log`. Called once
/// by whoever is handed the app data directory first; before that, and in tests, a note goes to
/// stderr alone.
pub fn init(dir: &Path, file: &str) {
    let stem = file.strip_suffix(".log").unwrap_or(file).to_string();
    let dir = dir.join("logs");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let today = Local::now().date_naive();
    prune(&dir, &stem, today, KEEP_DAYS);
    let _ = TARGET.set(Target { dir, stem });
}

/// The folder the day files live in, for "open logs folder".
pub fn path() -> Option<&'static Path> {
    TARGET.get().map(|t| t.dir.as_path())
}

/// One line, on stderr and in today's file. `who` is an account id or an area such as `ui`.
pub fn note(who: &str, line: &str) {
    let line = trim(line);
    eprintln!("[{who}] {line}");
    let Some(target) = TARGET.get() else { return };
    let now = Local::now();
    let today = now.date_naive();
    let entry = format!("{} {who} {line}\n", now.format("%Y-%m-%dT%H:%M:%S%.3f%:z"));
    let mut last = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    if *last != Some(today) {
        if last.is_some() {
            prune(&target.dir, &target.stem, today, KEEP_DAYS);
        }
        *last = Some(today);
    }
    let _ = append(&day_file(&target.dir, &target.stem, today), &entry, CAP_BYTES);
}

fn day_file(dir: &Path, stem: &str, day: NaiveDate) -> PathBuf {
    dir.join(format!("{stem}-{}.log", day.format("%Y-%m-%d")))
}

fn day_of(name: &str, stem: &str) -> Option<NaiveDate> {
    let date = name.strip_prefix(stem)?.strip_prefix('-')?.strip_suffix(".log")?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

fn prune(dir: &Path, stem: &str, today: NaiveDate, keep_days: i64) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(day) = name.to_str().and_then(|n| day_of(n, stem)) else { continue };
        if (today - day).num_days() >= keep_days {
            let _ = std::fs::remove_file(entry.path());
        }
    }
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

    #[tauri::command]
    pub fn log_dir() -> Result<String, String> {
        super::path()
            .map(|p| p.to_string_lossy().into_owned())
            .ok_or_else(|| "logging has not started".to_string())
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
