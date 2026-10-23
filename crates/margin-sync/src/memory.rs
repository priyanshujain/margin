use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use objstore::{
    BoxFuture, Capabilities, ETag, Error, ListPage, Object, ObjectMeta, ObjectStore, PutMode,
    Result, validate_key, validate_prefix,
};

/// An object store in memory, for tests of apps built on sync. `drive_like` behaves the way
/// Google Drive does for this crate: no conditional writes, and a delete moves the object to a
/// trash that is no longer listed or readable but can be counted.
pub struct MemoryStore {
    conditional: bool,
    objects: Mutex<BTreeMap<String, (Vec<u8>, ETag)>>,
    trash: Mutex<Vec<String>>,
    version: AtomicU64,
    requests: AtomicU64,
}

impl MemoryStore {
    pub fn new() -> MemoryStore {
        MemoryStore::with(true)
    }

    pub fn drive_like() -> MemoryStore {
        MemoryStore::with(false)
    }

    fn with(conditional: bool) -> MemoryStore {
        MemoryStore {
            conditional,
            objects: Mutex::new(BTreeMap::new()),
            trash: Mutex::new(Vec::new()),
            version: AtomicU64::new(0),
            requests: AtomicU64::new(0),
        }
    }

    pub fn keys(&self) -> Vec<String> {
        self.objects.lock().unwrap().keys().cloned().collect()
    }

    pub fn raw(&self, key: &str) -> Option<Vec<u8>> {
        self.objects
            .lock()
            .unwrap()
            .get(key)
            .map(|(b, _)| b.clone())
    }

    pub fn insert_raw(&self, key: &str, bytes: Vec<u8>) {
        let etag = self.next_etag();
        self.objects
            .lock()
            .unwrap()
            .insert(key.to_string(), (bytes, etag));
    }

    pub fn trashed(&self) -> Vec<String> {
        self.trash.lock().unwrap().clone()
    }

    pub fn requests(&self) -> u64 {
        self.requests.load(Ordering::SeqCst)
    }

    fn next_etag(&self) -> ETag {
        ETag::new(format!("v{}", self.version.fetch_add(1, Ordering::SeqCst)))
    }

    fn meta(key: &str, bytes: &[u8], etag: &ETag) -> ObjectMeta {
        ObjectMeta {
            key: key.to_string(),
            size: bytes.len() as u64,
            etag: etag.clone(),
            modified: None,
        }
    }
}

impl Default for MemoryStore {
    fn default() -> MemoryStore {
        MemoryStore::new()
    }
}

impl ObjectStore for MemoryStore {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            conditional_put: self.conditional,
        }
    }

    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'a, Result<ObjectMeta>> {
        Box::pin(async move {
            self.requests.fetch_add(1, Ordering::SeqCst);
            validate_key(key)?;
            if !self.conditional && mode != PutMode::Overwrite {
                return Err(Error::Unsupported {
                    operation: "conditional put",
                });
            }
            let mut objects = self.objects.lock().unwrap();
            let held = objects.get(key).map(|(_, etag)| etag.clone());
            let refused = match (&mode, &held) {
                (PutMode::IfNoneMatch, Some(_)) => true,
                (PutMode::IfMatch(want), Some(have)) => want != have,
                (PutMode::IfMatch(_), None) => true,
                _ => false,
            };
            if refused {
                return Err(Error::PreconditionFailed {
                    key: key.to_string(),
                });
            }
            let etag = self.next_etag();
            let meta = MemoryStore::meta(key, &bytes, &etag);
            objects.insert(key.to_string(), (bytes, etag));
            Ok(meta)
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>> {
        Box::pin(async move {
            self.requests.fetch_add(1, Ordering::SeqCst);
            validate_key(key)?;
            let objects = self.objects.lock().unwrap();
            let (bytes, etag) = objects.get(key).ok_or_else(|| Error::NotFound {
                key: key.to_string(),
            })?;
            Ok(Object {
                meta: MemoryStore::meta(key, bytes, etag),
                bytes: bytes.clone(),
            })
        })
    }

    fn head<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<ObjectMeta>> {
        Box::pin(async move { self.get(key).await.map(|object| object.meta) })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.requests.fetch_add(1, Ordering::SeqCst);
            validate_key(key)?;
            if self.objects.lock().unwrap().remove(key).is_some() && !self.conditional {
                self.trash.lock().unwrap().push(key.to_string());
            }
            Ok(())
        })
    }

    fn list_page<'a>(
        &'a self,
        prefix: &'a str,
        _token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>> {
        Box::pin(async move {
            self.requests.fetch_add(1, Ordering::SeqCst);
            validate_prefix(prefix)?;
            let objects = self.objects.lock().unwrap();
            Ok(ListPage {
                objects: objects
                    .iter()
                    .filter(|(key, _)| key.starts_with(prefix))
                    .map(|(key, (bytes, etag))| MemoryStore::meta(key, bytes, etag))
                    .collect(),
                next: None,
            })
        })
    }
}
