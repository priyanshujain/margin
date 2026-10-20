use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use objstore::{
    BoxFuture, Capabilities, ETag, Error, ListPage, Object, ObjectMeta, ObjectStore, PutMode,
    Result, validate_key, validate_prefix,
};
use sha2::{Digest, Sha256};

const TEMP_PREFIX: &str = ".objstore-";

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<FsStore> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(io_error)?;
        Ok(FsStore { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path(&self, key: &str) -> Result<PathBuf> {
        validate_key(key)?;
        let mut path = self.root.clone();
        for segment in key.split('/') {
            if is_foreign(segment) {
                return Err(Error::InvalidKey {
                    key: key.to_string(),
                    reason: "a segment looks like a hidden, temporary or sync conflict file",
                });
            }
            path.push(segment);
        }
        Ok(path)
    }

    async fn blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce(FsStore) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || work(store))
            .await
            .map_err(|e| Error::Io {
                message: e.to_string(),
            })?
    }

    fn put_sync(&self, key: &str, bytes: &[u8], mode: PutMode) -> Result<ObjectMeta> {
        let dest = self.path(key)?;
        let parent = dest.parent().expect("a key path has a parent");
        fs::create_dir_all(parent).map_err(io_error)?;
        let temp = write_temp(parent, bytes)?;
        let placed = match mode {
            PutMode::Overwrite => fs::rename(&temp, &dest).map_err(io_error),
            PutMode::IfNoneMatch => create_only(&temp, &dest, key),
            PutMode::IfMatch(expected) => replace_if(&temp, &dest, key, &expected),
        };
        let _ = fs::remove_file(&temp);
        placed?;
        sync_dir(parent);
        meta(key, &dest, etag_of(bytes))
    }

    fn get_sync(&self, key: &str) -> Result<Object> {
        let path = self.path(key)?;
        let bytes = fs::read(&path).map_err(|e| missing(e, key))?;
        Ok(Object {
            meta: meta(key, &path, etag_of(&bytes))?,
            bytes,
        })
    }

    fn head_sync(&self, key: &str) -> Result<ObjectMeta> {
        let path = self.path(key)?;
        let etag = hash_file(&path).map_err(|e| missing(e, key))?;
        meta(key, &path, etag)
    }

    fn delete_sync(&self, key: &str) -> Result<()> {
        let path = self.path(key)?;
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(io_error(e)),
        }
        let mut dir = path.parent();
        while let Some(at) = dir {
            if at == self.root || fs::remove_dir(at).is_err() {
                break;
            }
            dir = at.parent();
        }
        Ok(())
    }

    fn list_sync(&self, prefix: &str) -> Result<ListPage> {
        validate_prefix(prefix)?;
        let base = match prefix.rfind('/') {
            Some(at) => &prefix[..at],
            None => "",
        };
        let mut start = self.root.clone();
        for segment in base.split('/').filter(|segment| !segment.is_empty()) {
            if is_foreign(segment) {
                return Ok(ListPage::default());
            }
            start.push(segment);
        }
        let mut keys = Vec::new();
        walk(&start, base, &mut keys)?;
        keys.retain(|(key, _)| key.starts_with(prefix));
        keys.sort();
        let mut objects = Vec::with_capacity(keys.len());
        for (key, path) in keys {
            match hash_file(&path) {
                Ok(etag) => objects.push(meta(&key, &path, etag)?),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_error(e)),
            }
        }
        Ok(ListPage {
            objects,
            next: None,
        })
    }
}

impl ObjectStore for FsStore {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            conditional_put: true,
        }
    }

    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'a, Result<ObjectMeta>> {
        let key = key.to_string();
        Box::pin(self.blocking(move |store| store.put_sync(&key, &bytes, mode)))
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>> {
        let key = key.to_string();
        Box::pin(self.blocking(move |store| store.get_sync(&key)))
    }

    fn head<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<ObjectMeta>> {
        let key = key.to_string();
        Box::pin(self.blocking(move |store| store.head_sync(&key)))
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
        let key = key.to_string();
        Box::pin(self.blocking(move |store| store.delete_sync(&key)))
    }

    fn list_page<'a>(
        &'a self,
        prefix: &'a str,
        _token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>> {
        let prefix = prefix.to_string();
        Box::pin(self.blocking(move |store| store.list_sync(&prefix)))
    }
}

pub fn is_foreign(name: &str) -> bool {
    name.starts_with('.')
        || name.starts_with('~')
        || name.eq_ignore_ascii_case("desktop.ini")
        || name.eq_ignore_ascii_case("thumbs.db")
        || name.starts_with("Icon\r")
        || name.contains(" (conflicted copy")
        || name.contains("'s conflicted copy")
        || name.contains(".sync-conflict-")
        || name.contains(" (Case Conflict")
        || name.contains(" (Selective Sync Conflict")
        || name.ends_with(".icloud")
        || name.ends_with(".partial")
        || name.ends_with(".tmp")
}

fn walk(dir: &Path, key_base: &str, out: &mut Vec<(String, PathBuf)>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(());
        }
        Err(e) => return Err(io_error(e)),
    };
    for entry in entries {
        let entry = entry.map_err(io_error)?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if is_foreign(&name) {
            continue;
        }
        let key = if key_base.is_empty() {
            name
        } else {
            format!("{key_base}/{name}")
        };
        let kind = entry.file_type().map_err(io_error)?;
        if kind.is_dir() {
            walk(&entry.path(), &key, out)?;
        } else if kind.is_file() {
            out.push((key, entry.path()));
        }
    }
    Ok(())
}

fn write_temp(dir: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let path = dir.join(format!(
        "{TEMP_PREFIX}{}-{}-{nanos}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(io_error)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    if let Err(e) = written {
        let _ = fs::remove_file(&path);
        return Err(io_error(e));
    }
    Ok(path)
}

fn create_only(temp: &Path, dest: &Path, key: &str) -> Result<()> {
    match fs::hard_link(temp, dest) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(Error::PreconditionFailed {
            key: key.to_string(),
        }),
        Err(e) => Err(io_error(e)),
    }
}

fn replace_if(temp: &Path, dest: &Path, key: &str, expected: &ETag) -> Result<()> {
    let failed = || Error::PreconditionFailed {
        key: key.to_string(),
    };
    let swap = temp.with_extension("swap");
    match fs::rename(dest, &swap) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(failed()),
        Err(e) => return Err(io_error(e)),
    }
    let current = hash_file(&swap);
    let outcome = match current {
        Ok(etag) if etag == *expected => create_only(temp, dest, key),
        Ok(_) => {
            let _ = fs::hard_link(&swap, dest);
            Err(failed())
        }
        Err(e) => {
            let _ = fs::hard_link(&swap, dest);
            Err(io_error(e))
        }
    };
    let _ = fs::remove_file(&swap);
    outcome
}

fn sync_dir(dir: &Path) {
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
}

fn etag_of(bytes: &[u8]) -> ETag {
    ETag::new(hex(&Sha256::digest(bytes)))
}

fn hash_file(path: &Path) -> io::Result<ETag> {
    let mut file = File::open(path)?;
    if file.metadata()?.is_dir() {
        return Err(io::Error::from(io::ErrorKind::NotFound));
    }
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(ETag::new(hex(&hasher.finalize())))
}

fn meta(key: &str, path: &Path, etag: ETag) -> Result<ObjectMeta> {
    let metadata = fs::metadata(path).map_err(|e| missing(e, key))?;
    Ok(ObjectMeta {
        key: key.to_string(),
        size: metadata.len(),
        etag,
        modified: metadata.modified().ok(),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn missing(e: io::Error, key: &str) -> Error {
    match e.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::IsADirectory | io::ErrorKind::NotADirectory => {
            Error::NotFound {
                key: key.to_string(),
            }
        }
        _ => io_error(e),
    }
}

fn io_error(e: io::Error) -> Error {
    Error::Io {
        message: e.to_string(),
    }
}
