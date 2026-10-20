use crate::{
    BoxFuture, Capabilities, Error, ListPage, Object, ObjectMeta, ObjectStore, PutMode, Result,
    validate_key,
};

pub struct Prefixed<S> {
    inner: S,
    prefix: String,
}

impl<S: ObjectStore> Prefixed<S> {
    pub fn new(inner: S, prefix: &str) -> Result<Prefixed<S>> {
        let trimmed = prefix.trim_end_matches('/');
        validate_key(trimmed)?;
        Ok(Prefixed {
            inner,
            prefix: format!("{trimmed}/"),
        })
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    pub fn into_inner(self) -> S {
        self.inner
    }

    fn full(&self, key: &str) -> Result<String> {
        validate_key(key)?;
        Ok(format!("{}{key}", self.prefix))
    }

    fn strip(&self, mut meta: ObjectMeta) -> Result<ObjectMeta> {
        match meta.key.strip_prefix(&self.prefix) {
            Some(rest) => {
                meta.key = rest.to_string();
                Ok(meta)
            }
            None => Err(Error::Backend {
                status: None,
                message: format!("the store returned {} outside {}", meta.key, self.prefix),
            }),
        }
    }

    fn restore(&self, error: Error, key: &str) -> Error {
        match error {
            Error::NotFound { .. } => Error::NotFound {
                key: key.to_string(),
            },
            Error::PreconditionFailed { .. } => Error::PreconditionFailed {
                key: key.to_string(),
            },
            other => other,
        }
    }
}

impl<S: ObjectStore> ObjectStore for Prefixed<S> {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }

    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'a, Result<ObjectMeta>> {
        Box::pin(async move {
            let full = self.full(key)?;
            let meta = self
                .inner
                .put(&full, bytes, mode)
                .await
                .map_err(|e| self.restore(e, key))?;
            self.strip(meta)
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>> {
        Box::pin(async move {
            let full = self.full(key)?;
            let object = self
                .inner
                .get(&full)
                .await
                .map_err(|e| self.restore(e, key))?;
            Ok(Object {
                meta: self.strip(object.meta)?,
                bytes: object.bytes,
            })
        })
    }

    fn head<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<ObjectMeta>> {
        Box::pin(async move {
            let full = self.full(key)?;
            let meta = self
                .inner
                .head(&full)
                .await
                .map_err(|e| self.restore(e, key))?;
            self.strip(meta)
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let full = self.full(key)?;
            self.inner
                .delete(&full)
                .await
                .map_err(|e| self.restore(e, key))
        })
    }

    fn list_page<'a>(
        &'a self,
        prefix: &'a str,
        token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>> {
        Box::pin(async move {
            crate::validate_prefix(prefix)?;
            let full = format!("{}{prefix}", self.prefix);
            let page = self.inner.list_page(&full, token).await?;
            Ok(ListPage {
                objects: page
                    .objects
                    .into_iter()
                    .map(|meta| self.strip(meta))
                    .collect::<Result<_>>()?,
                next: page.next,
            })
        })
    }
}
