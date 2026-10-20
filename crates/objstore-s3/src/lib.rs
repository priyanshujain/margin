use std::time::{Duration, SystemTime};

use objstore::{
    BoxFuture, Capabilities, ETag, Error, ListPage, Object, ObjectMeta, ObjectStore, PutMode,
    Result, validate_key, validate_prefix,
};
use reqwest::{Method, Response, StatusCode, header};
use sha2::{Digest, Sha256};

mod sigv4;
mod xml;

pub use sigv4::{authorization, encode, hmac_sha256, signing_key};

#[derive(Debug, Clone)]
pub struct Config {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret: String,
    pub allow_http: bool,
    pub page_size: Option<u32>,
}

impl Config {
    pub fn new(
        endpoint: impl Into<String>,
        region: impl Into<String>,
        bucket: impl Into<String>,
        access_key: impl Into<String>,
        secret: impl Into<String>,
    ) -> Config {
        Config {
            endpoint: endpoint.into(),
            region: region.into(),
            bucket: bucket.into(),
            access_key: access_key.into(),
            secret: secret.into(),
            allow_http: false,
            page_size: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct S3Store {
    config: Config,
    host: String,
    http: reqwest::Client,
}

impl S3Store {
    pub fn new(mut config: Config) -> Result<S3Store> {
        config.endpoint = config.endpoint.trim().trim_end_matches('/').to_string();
        let invalid = |message: String| Error::Backend {
            status: None,
            message,
        };
        let url = url::Url::parse(&config.endpoint).map_err(|e| {
            invalid(format!(
                "{} is not an endpoint address: {e}",
                config.endpoint
            ))
        })?;
        match url.scheme() {
            "https" => {}
            "http" if config.allow_http => {}
            _ => {
                return Err(invalid(
                    "the endpoint has to start with https://".to_string(),
                ));
            }
        }
        let host = url
            .host_str()
            .ok_or_else(|| invalid(format!("{} is not an endpoint address", config.endpoint)))?;
        let host = match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        };
        if config.bucket.is_empty() || config.bucket.contains('/') {
            return Err(invalid(
                "a bucket name is one word with no slashes".to_string(),
            ));
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| Error::Transport {
                message: e.to_string(),
            })?;
        Ok(S3Store { config, host, http })
    }

    fn path_for(&self, key: &str) -> String {
        format!(
            "/{}/{}",
            encode(&self.config.bucket, false),
            encode(key, true)
        )
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        extra: &[(&str, String)],
        body: Vec<u8>,
    ) -> Result<Response> {
        let payload_sha = hex(&Sha256::digest(&body));
        let amz_date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let canonical_query = sigv4::canonical_query(query);
        let mut headers = vec![
            ("host", self.host.clone()),
            ("x-amz-content-sha256", payload_sha.clone()),
            ("x-amz-date", amz_date.clone()),
        ];
        headers.extend(extra.iter().cloned());
        let authorization = authorization(
            &self.config.access_key,
            &self.config.secret,
            &self.config.region,
            "s3",
            method.as_str(),
            path,
            &canonical_query,
            &headers,
            &payload_sha,
            &amz_date,
        );
        let mut url = format!("{}{path}", self.config.endpoint);
        if !canonical_query.is_empty() {
            url.push('?');
            url.push_str(&canonical_query);
        }
        let mut request = self
            .http
            .request(method, url)
            .header(header::AUTHORIZATION, authorization)
            .body(body);
        for (name, value) in &headers[1..] {
            request = request.header(*name, value);
        }
        request.send().await.map_err(|e| Error::Transport {
            message: e.to_string(),
        })
    }
}

async fn refused(key: &str, resp: Response) -> Error {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let code = xml::tag(&body, "Code").unwrap_or_default();
    match status {
        StatusCode::NOT_FOUND if code != "NoSuchBucket" => Error::NotFound {
            key: key.to_string(),
        },
        StatusCode::PRECONDITION_FAILED => Error::PreconditionFailed {
            key: key.to_string(),
        },
        StatusCode::CONFLICT if code == "ConditionalRequestConflict" => Error::PreconditionFailed {
            key: key.to_string(),
        },
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Error::Unauthorized {
            message: xml::tag(&body, "Message").unwrap_or(code),
        },
        _ => Error::Backend {
            status: Some(status.as_u16()),
            message: xml::tag(&body, "Message").unwrap_or(code),
        },
    }
}

fn etag_from(resp: &Response) -> ETag {
    let raw = resp
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    unquote(raw)
}

fn unquote(raw: &str) -> ETag {
    ETag::new(raw.trim().trim_start_matches("W/").trim_matches('"'))
}

fn meta_from(key: &str, resp: &Response) -> ObjectMeta {
    let headers = resp.headers();
    ObjectMeta {
        key: key.to_string(),
        size: headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()?.parse().ok())
            .unwrap_or_default(),
        etag: etag_from(resp),
        modified: headers
            .get(header::LAST_MODIFIED)
            .and_then(|value| chrono::DateTime::parse_from_rfc2822(value.to_str().ok()?).ok())
            .map(SystemTime::from),
    }
}

impl ObjectStore for S3Store {
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
        Box::pin(async move {
            validate_key(key)?;
            let size = bytes.len() as u64;
            let extra = match &mode {
                PutMode::Overwrite => vec![],
                PutMode::IfNoneMatch => vec![("if-none-match", "*".to_string())],
                PutMode::IfMatch(etag) => vec![("if-match", format!("\"{}\"", etag.as_str()))],
            };
            let resp = self
                .send(Method::PUT, &self.path_for(key), &[], &extra, bytes)
                .await?;
            if !resp.status().is_success() {
                let error = refused(key, resp).await;
                return Err(match (error, mode) {
                    (Error::NotFound { key }, PutMode::IfMatch(_)) => {
                        Error::PreconditionFailed { key }
                    }
                    (error, _) => error,
                });
            }
            Ok(ObjectMeta {
                key: key.to_string(),
                size,
                etag: etag_from(&resp),
                modified: None,
            })
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>> {
        Box::pin(async move {
            validate_key(key)?;
            let resp = self
                .send(Method::GET, &self.path_for(key), &[], &[], Vec::new())
                .await?;
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
            let resp = self
                .send(Method::HEAD, &self.path_for(key), &[], &[], Vec::new())
                .await?;
            if !resp.status().is_success() {
                return Err(refused(key, resp).await);
            }
            Ok(meta_from(key, &resp))
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            validate_key(key)?;
            let resp = self
                .send(Method::DELETE, &self.path_for(key), &[], &[], Vec::new())
                .await?;
            if resp.status().is_success() || resp.status() == StatusCode::NOT_FOUND {
                return Ok(());
            }
            Err(refused(key, resp).await)
        })
    }

    fn list_page<'a>(
        &'a self,
        prefix: &'a str,
        token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>> {
        Box::pin(async move {
            validate_prefix(prefix)?;
            let path = format!("/{}", encode(&self.config.bucket, false));
            let page_size = self.config.page_size.map(|size| size.to_string());
            let mut query = vec![("list-type", "2"), ("prefix", prefix)];
            if let Some(token) = &token {
                query.push(("continuation-token", token.as_str()));
            }
            if let Some(size) = &page_size {
                query.push(("max-keys", size.as_str()));
            }
            let resp = self
                .send(Method::GET, &path, &query, &[], Vec::new())
                .await?;
            if !resp.status().is_success() {
                return Err(refused(prefix, resp).await);
            }
            let body = resp.text().await.map_err(|e| Error::Transport {
                message: e.to_string(),
            })?;
            let objects = xml::tags(&body, "Contents")
                .iter()
                .map(|entry| ObjectMeta {
                    key: xml::unescape(&xml::tag(entry, "Key").unwrap_or_default()),
                    size: xml::tag(entry, "Size")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_default(),
                    etag: unquote(&xml::unescape(&xml::tag(entry, "ETag").unwrap_or_default())),
                    modified: xml::tag(entry, "LastModified")
                        .and_then(|at| chrono::DateTime::parse_from_rfc3339(&at).ok())
                        .map(SystemTime::from),
                })
                .collect();
            let next = match xml::tag(&body, "IsTruncated").as_deref() {
                Some("true") => xml::tag(&body, "NextContinuationToken").map(|t| xml::unescape(&t)),
                _ => None,
            };
            Ok(ListPage { objects, next })
        })
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
