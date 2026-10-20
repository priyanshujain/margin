use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

mod prefixed;

pub use prefixed::Prefixed;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ETag(String);

impl ETag {
    pub fn new(value: impl Into<String>) -> ETag {
        ETag(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ETag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMeta {
    pub key: String,
    pub size: u64,
    pub etag: ETag,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    pub meta: ObjectMeta,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PutMode {
    #[default]
    Overwrite,
    IfNoneMatch,
    IfMatch(ETag),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListPage {
    pub objects: Vec<ObjectMeta>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub conditional_put: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotFound {
        key: String,
    },
    PreconditionFailed {
        key: String,
    },
    InvalidKey {
        key: String,
        reason: &'static str,
    },
    Unsupported {
        operation: &'static str,
    },
    Unauthorized {
        message: String,
    },
    Transport {
        message: String,
    },
    Backend {
        status: Option<u16>,
        message: String,
    },
    Io {
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound { key } => write!(f, "{key} does not exist"),
            Error::PreconditionFailed { key } => {
                write!(f, "{key} changed since it was last read")
            }
            Error::InvalidKey { key, reason } => write!(f, "{key:?} is not a valid key: {reason}"),
            Error::Unsupported { operation } => {
                write!(f, "this store does not support {operation}")
            }
            Error::Unauthorized { message } => {
                write!(f, "the store refused the credentials: {message}")
            }
            Error::Transport { message } => write!(f, "could not reach the store: {message}"),
            Error::Backend {
                status: Some(status),
                message,
            } => write!(f, "the store answered {status}: {message}"),
            Error::Backend {
                status: None,
                message,
            } => write!(f, "the store failed: {message}"),
            Error::Io { message } => write!(f, "file system error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

pub trait ObjectStore: Send + Sync {
    fn capabilities(&self) -> Capabilities;

    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'a, Result<ObjectMeta>>;

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>>;

    fn head<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<ObjectMeta>>;

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>>;

    fn list_page<'a>(
        &'a self,
        prefix: &'a str,
        token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>>;

    fn list<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, Result<Vec<ObjectMeta>>> {
        Box::pin(async move {
            let mut out = Vec::new();
            let mut token = None;
            loop {
                let page = self.list_page(prefix, token).await?;
                out.extend(page.objects);
                match page.next {
                    Some(next) => token = Some(next),
                    None => return Ok(out),
                }
            }
        })
    }
}

impl<S: ObjectStore + ?Sized> ObjectStore for std::sync::Arc<S> {
    fn capabilities(&self) -> Capabilities {
        (**self).capabilities()
    }
    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'a, Result<ObjectMeta>> {
        (**self).put(key, bytes, mode)
    }
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>> {
        (**self).get(key)
    }
    fn head<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<ObjectMeta>> {
        (**self).head(key)
    }
    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
        (**self).delete(key)
    }
    fn list_page<'a>(
        &'a self,
        prefix: &'a str,
        token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>> {
        (**self).list_page(prefix, token)
    }
}

impl<S: ObjectStore + ?Sized> ObjectStore for Box<S> {
    fn capabilities(&self) -> Capabilities {
        (**self).capabilities()
    }
    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'a, Result<ObjectMeta>> {
        (**self).put(key, bytes, mode)
    }
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>> {
        (**self).get(key)
    }
    fn head<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<ObjectMeta>> {
        (**self).head(key)
    }
    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
        (**self).delete(key)
    }
    fn list_page<'a>(
        &'a self,
        prefix: &'a str,
        token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>> {
        (**self).list_page(prefix, token)
    }
}

impl<S: ObjectStore + ?Sized> ObjectStore for &S {
    fn capabilities(&self) -> Capabilities {
        (**self).capabilities()
    }
    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'a, Result<ObjectMeta>> {
        (**self).put(key, bytes, mode)
    }
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>> {
        (**self).get(key)
    }
    fn head<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<ObjectMeta>> {
        (**self).head(key)
    }
    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
        (**self).delete(key)
    }
    fn list_page<'a>(
        &'a self,
        prefix: &'a str,
        token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>> {
        (**self).list_page(prefix, token)
    }
}

pub fn validate_key(key: &str) -> Result<()> {
    let invalid = |reason| {
        Err(Error::InvalidKey {
            key: key.to_string(),
            reason,
        })
    };
    if key.is_empty() {
        return invalid("it is empty");
    }
    if key.len() > 1024 {
        return invalid("it is longer than 1024 bytes");
    }
    if key.contains(['\\', '\0']) || key.chars().any(char::is_control) {
        return invalid("it contains a backslash or a control character");
    }
    for segment in key.split('/') {
        match segment {
            "" => return invalid("it has an empty path segment or a leading or trailing slash"),
            "." | ".." => return invalid("it has a . or .. segment"),
            _ => {}
        }
    }
    Ok(())
}

pub fn validate_prefix(prefix: &str) -> Result<()> {
    if prefix.is_empty() {
        return Ok(());
    }
    validate_key(prefix.strip_suffix('/').unwrap_or(prefix))
}
