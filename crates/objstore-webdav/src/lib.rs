use std::time::{Duration, SystemTime};

use objstore::{
    BoxFuture, Capabilities, ETag, Error, ListPage, Object, ObjectMeta, ObjectStore, PutMode,
    Result, validate_key, validate_prefix,
};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use reqwest::{Method, RequestBuilder, Response, StatusCode, header};

mod multistatus;

const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/><d:getetag/><d:getcontentlength/><d:getlastmodified/></d:prop></d:propfind>"#;

#[derive(Debug, Clone)]
pub struct Config {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub conditional_put: bool,
    pub allow_http: bool,
}

impl Config {
    pub fn new(url: impl Into<String>) -> Config {
        Config {
            url: url.into(),
            username: None,
            password: None,
            conditional_put: false,
            allow_http: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebDavStore {
    config: Config,
    base_path: String,
    http: reqwest::Client,
}

impl WebDavStore {
    pub fn new(mut config: Config) -> Result<WebDavStore> {
        let invalid = |message: String| Error::Backend {
            status: None,
            message,
        };
        if !config.url.ends_with('/') {
            config.url.push('/');
        }
        let url = url::Url::parse(&config.url)
            .map_err(|e| invalid(format!("{} is not a WebDAV address: {e}", config.url)))?;
        match url.scheme() {
            "https" => {}
            "http" if config.allow_http => {}
            _ => {
                return Err(invalid(
                    "the WebDAV address has to start with https://".to_string(),
                ));
            }
        }
        let base_path = percent_decode_str(url.path())
            .decode_utf8_lossy()
            .into_owned();
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| Error::Transport {
                message: e.to_string(),
            })?;
        Ok(WebDavStore {
            config,
            base_path,
            http,
        })
    }

    fn url_for(&self, path: &str) -> String {
        let encoded: Vec<String> = path
            .split('/')
            .map(|segment| utf8_percent_encode(segment, SEGMENT).to_string())
            .collect();
        format!("{}{}", self.config.url, encoded.join("/"))
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let request = self.http.request(method, self.url_for(path));
        match &self.config.username {
            Some(user) => request.basic_auth(user, self.config.password.as_ref()),
            None => request,
        }
    }

    async fn send(&self, request: RequestBuilder) -> Result<Response> {
        request.send().await.map_err(|e| Error::Transport {
            message: e.to_string(),
        })
    }

    async fn make_parents(&self, key: &str) -> Result<()> {
        let segments: Vec<&str> = key.split('/').collect();
        for depth in 1..segments.len() {
            let collection = format!("{}/", segments[..depth].join("/"));
            let mkcol = Method::from_bytes(b"MKCOL").expect("MKCOL is a valid method");
            let resp = self.send(self.request(mkcol, &collection)).await?;
            match resp.status() {
                status if status.is_success() => {}
                StatusCode::METHOD_NOT_ALLOWED => {}
                _ => return Err(refused(key, resp).await),
            }
        }
        Ok(())
    }

    async fn put_once(&self, key: &str, bytes: &[u8], mode: &PutMode) -> Result<Response> {
        let mut request = self.request(Method::PUT, key).body(bytes.to_vec());
        request = match mode {
            PutMode::Overwrite => request,
            PutMode::IfNoneMatch => request.header(header::IF_NONE_MATCH, "*"),
            PutMode::IfMatch(etag) => request.header(header::IF_MATCH, quote(etag)),
        };
        self.send(request).await
    }

    async fn propfind(&self, collection: &str) -> Result<Vec<multistatus::Entry>> {
        let propfind = Method::from_bytes(b"PROPFIND").expect("PROPFIND is a valid method");
        let resp = self
            .send(
                self.request(propfind, collection)
                    .header("depth", "1")
                    .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
                    .body(PROPFIND_BODY),
            )
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        if resp.status() != StatusCode::MULTI_STATUS {
            return Err(refused(collection, resp).await);
        }
        let body = resp.text().await.map_err(|e| Error::Transport {
            message: e.to_string(),
        })?;
        multistatus::parse(&body).map_err(|message| Error::Backend {
            status: None,
            message,
        })
    }

    fn key_of(&self, href: &str) -> Option<String> {
        let path = match url::Url::parse(href) {
            Ok(url) => url.path().to_string(),
            Err(_) => href.to_string(),
        };
        let decoded = percent_decode_str(&path).decode_utf8().ok()?.into_owned();
        decoded
            .strip_prefix(&self.base_path)
            .map(|rest| rest.trim_end_matches('/').to_string())
    }
}

async fn refused(key: &str, resp: Response) -> Error {
    let status = resp.status();
    let message = resp
        .text()
        .await
        .unwrap_or_default()
        .chars()
        .take(300)
        .collect();
    match status {
        StatusCode::NOT_FOUND => Error::NotFound {
            key: key.to_string(),
        },
        StatusCode::PRECONDITION_FAILED => Error::PreconditionFailed {
            key: key.to_string(),
        },
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Error::Unauthorized { message },
        _ => Error::Backend {
            status: Some(status.as_u16()),
            message,
        },
    }
}

fn quote(etag: &ETag) -> String {
    format!("\"{}\"", etag.as_str())
}

fn unquote(raw: &str) -> ETag {
    ETag::new(raw.trim().trim_start_matches("W/").trim_matches('"'))
}

fn parse_date(raw: &str) -> Option<SystemTime> {
    chrono::DateTime::parse_from_rfc2822(raw.trim())
        .ok()
        .map(SystemTime::from)
}

fn meta_from(key: &str, resp: &Response) -> ObjectMeta {
    let headers = resp.headers();
    let text = |name| {
        headers
            .get(name)
            .and_then(|value: &header::HeaderValue| value.to_str().ok())
    };
    ObjectMeta {
        key: key.to_string(),
        size: text(header::CONTENT_LENGTH)
            .and_then(|v| v.parse().ok())
            .unwrap_or_default(),
        etag: unquote(text(header::ETAG).unwrap_or_default()),
        modified: text(header::LAST_MODIFIED).and_then(parse_date),
    }
}

impl ObjectStore for WebDavStore {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            conditional_put: self.config.conditional_put,
        }
    }

    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: Vec<u8>,
        mode: PutMode,
    ) -> BoxFuture<'a, Result<ObjectMeta>> {
        Box::pin(async move {
            validate_key(key)?;
            if mode != PutMode::Overwrite && !self.config.conditional_put {
                return Err(Error::Unsupported {
                    operation: "conditional put",
                });
            }
            if matches!(mode, PutMode::IfMatch(_))
                && let Err(Error::NotFound { .. }) = self.head(key).await
            {
                return Err(Error::PreconditionFailed {
                    key: key.to_string(),
                });
            }
            let mut resp = self.put_once(key, &bytes, &mode).await?;
            if resp.status() == StatusCode::CONFLICT && key.contains('/') {
                self.make_parents(key).await?;
                resp = self.put_once(key, &bytes, &mode).await?;
            }
            if !resp.status().is_success() {
                return Err(match refused(key, resp).await {
                    Error::NotFound { .. }
                    | Error::Backend {
                        status: Some(409), ..
                    } if matches!(mode, PutMode::IfMatch(_)) => Error::PreconditionFailed {
                        key: key.to_string(),
                    },
                    other => other,
                });
            }
            let etag = resp
                .headers()
                .get(header::ETAG)
                .and_then(|v| v.to_str().ok())
                .map(unquote);
            let etag = match etag {
                Some(etag) => etag,
                None => self.head(key).await?.etag,
            };
            Ok(ObjectMeta {
                key: key.to_string(),
                size: bytes.len() as u64,
                etag,
                modified: None,
            })
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>> {
        Box::pin(async move {
            validate_key(key)?;
            let resp = self.send(self.request(Method::GET, key)).await?;
            if !resp.status().is_success() {
                return Err(refused(key, resp).await);
            }
            let meta = meta_from(key, &resp);
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| Error::Transport {
                    message: e.to_string(),
                })?
                .to_vec();
            Ok(Object {
                meta: ObjectMeta {
                    size: bytes.len() as u64,
                    ..meta
                },
                bytes,
            })
        })
    }

    fn head<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<ObjectMeta>> {
        Box::pin(async move {
            validate_key(key)?;
            let resp = self.send(self.request(Method::HEAD, key)).await?;
            if !resp.status().is_success() {
                return Err(refused(key, resp).await);
            }
            Ok(meta_from(key, &resp))
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            validate_key(key)?;
            let resp = self.send(self.request(Method::DELETE, key)).await?;
            if resp.status().is_success() || resp.status() == StatusCode::NOT_FOUND {
                return Ok(());
            }
            Err(refused(key, resp).await)
        })
    }

    fn list_page<'a>(
        &'a self,
        prefix: &'a str,
        _token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>> {
        Box::pin(async move {
            validate_prefix(prefix)?;
            let start = match prefix.rfind('/') {
                Some(at) => prefix[..=at].to_string(),
                None => String::new(),
            };
            let mut pending = vec![start];
            let mut objects = Vec::new();
            while let Some(collection) = pending.pop() {
                for entry in self.propfind(&collection).await? {
                    let Some(key) = self.key_of(&entry.href) else {
                        continue;
                    };
                    if key.is_empty() || format!("{key}/") == collection {
                        continue;
                    }
                    if entry.collection {
                        let child = format!("{key}/");
                        if child.starts_with(prefix) || prefix.starts_with(&child) {
                            pending.push(child);
                        }
                    } else if key.starts_with(prefix) {
                        objects.push(ObjectMeta {
                            key,
                            size: entry.size,
                            etag: unquote(&entry.etag),
                            modified: entry.modified.as_deref().and_then(parse_date),
                        });
                    }
                }
            }
            objects.sort_by(|a, b| a.key.cmp(&b.key));
            Ok(ListPage {
                objects,
                next: None,
            })
        })
    }
}
