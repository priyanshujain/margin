use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use objstore::{
    BoxFuture, Capabilities, ETag, Error, ListPage, Object, ObjectMeta, ObjectStore, PutMode,
    Result, validate_key, validate_prefix,
};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use rand::Rng;
use reqwest::{Response, StatusCode};
use serde::Deserialize;
use tokio::sync::Mutex;

pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
pub const GOOGLE_API: &str = "https://www.googleapis.com";

const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const FILE_FIELDS: &str = "id,name,mimeType,size,modifiedTime,md5Checksum,version";

pub type TokenProvider =
    Arc<dyn Fn() -> BoxFuture<'static, std::result::Result<String, String>> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct Config {
    pub root: Vec<String>,
    pub api_base: String,
}

impl Config {
    pub fn new<I, S>(root: I) -> Config
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Config {
            root: root.into_iter().map(Into::into).collect(),
            api_base: GOOGLE_API.to_string(),
        }
    }
}

pub struct DriveStore {
    config: Config,
    token: TokenProvider,
    http: reqwest::Client,
    folders: Mutex<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    mime_type: String,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    modified_time: Option<String>,
    #[serde(default)]
    md5_checksum: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

impl File {
    fn meta(&self, key: String) -> ObjectMeta {
        ObjectMeta {
            key,
            size: self
                .size
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            etag: ETag::new(
                self.md5_checksum
                    .clone()
                    .or_else(|| self.version.clone())
                    .unwrap_or_default(),
            ),
            modified: self
                .modified_time
                .as_deref()
                .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
                .map(SystemTime::from),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileList {
    #[serde(default)]
    files: Vec<File>,
    #[serde(default)]
    next_page_token: Option<String>,
}

pub fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

impl DriveStore {
    pub fn new(config: Config, token: TokenProvider) -> Result<DriveStore> {
        if config.root.is_empty() || config.root.iter().any(|name| name.is_empty()) {
            return Err(Error::Backend {
                status: None,
                message: "the Drive root needs at least one non-empty folder name".to_string(),
            });
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| Error::Transport {
                message: e.to_string(),
            })?;
        Ok(DriveStore {
            config,
            token,
            http,
            folders: Mutex::new(HashMap::new()),
        })
    }

    fn files_url(&self) -> String {
        format!("{}/drive/v3/files", self.config.api_base)
    }

    fn upload_url(&self) -> String {
        format!("{}/upload/drive/v3/files", self.config.api_base)
    }

    async fn token(&self) -> Result<String> {
        (self.token)()
            .await
            .map_err(|message| Error::Unauthorized { message })
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> Result<Response> {
        let token = self.token().await?;
        request
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| Error::Transport {
                message: e.to_string(),
            })
    }

    async fn query(&self, q: &str) -> Result<Vec<File>> {
        let fields = format!("nextPageToken,files({FILE_FIELDS})");
        let mut files = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut params = vec![
                ("q", q),
                ("fields", fields.as_str()),
                ("spaces", "drive"),
                ("pageSize", "1000"),
            ];
            if let Some(token) = &page_token {
                params.push(("pageToken", token.as_str()));
            }
            let resp = self
                .send(self.http.get(self.files_url()).query(&params))
                .await?;
            let page: FileList = read_json(q, resp).await?;
            files.extend(page.files);
            match page.next_page_token {
                Some(token) if !token.is_empty() => page_token = Some(token),
                _ => return Ok(files),
            }
        }
    }

    async fn folder(&self, segments: &[&str], create: bool) -> Result<Option<String>> {
        let mut parent: Option<String> = None;
        let mut path = String::new();
        for segment in self
            .config
            .root
            .iter()
            .map(String::as_str)
            .chain(segments.iter().copied())
        {
            path.push('/');
            path.push_str(segment);
            if let Some(id) = self.folders.lock().await.get(&path).cloned() {
                parent = Some(id);
                continue;
            }
            let mut q = format!(
                "name = '{}' and mimeType = '{FOLDER_MIME}' and trashed = false",
                escape(segment)
            );
            if let Some(parent) = &parent {
                q.push_str(&format!(" and '{}' in parents", escape(parent)));
            }
            let id = match self.query(&q).await?.into_iter().next() {
                Some(found) => found.id,
                None if create => self.create_folder(segment, parent.as_deref()).await?,
                None => return Ok(None),
            };
            self.folders.lock().await.insert(path.clone(), id.clone());
            parent = Some(id);
        }
        Ok(parent)
    }

    async fn create_folder(&self, name: &str, parent: Option<&str>) -> Result<String> {
        let mut body = serde_json::json!({ "name": name, "mimeType": FOLDER_MIME });
        if let Some(parent) = parent {
            body["parents"] = serde_json::json!([parent]);
        }
        let resp = self
            .send(
                self.http
                    .post(self.files_url())
                    .query(&[("fields", "id")])
                    .json(&body),
            )
            .await?;
        let created: File = read_json(name, resp).await?;
        Ok(created.id)
    }

    async fn find(
        &self,
        key: &str,
        create_folders: bool,
    ) -> Result<(Option<String>, Option<File>)> {
        let (dirs, name) = split(key);
        let Some(folder) = self.folder(&dirs, create_folders).await? else {
            return Ok((None, None));
        };
        let q = format!(
            "'{}' in parents and name = '{}' and mimeType != '{FOLDER_MIME}' and trashed = false",
            escape(&folder),
            escape(name)
        );
        let file = self.query(&q).await?.into_iter().next();
        Ok((Some(folder), file))
    }

    async fn walk(
        &self,
        folder: String,
        base: String,
        prefix: &str,
        out: &mut Vec<ObjectMeta>,
    ) -> Result<()> {
        let mut pending = vec![(folder, base)];
        while let Some((folder, base)) = pending.pop() {
            let q = format!("'{}' in parents and trashed = false", escape(&folder));
            for file in self.query(&q).await? {
                let key = if base.is_empty() {
                    file.name.clone()
                } else {
                    format!("{base}/{}", file.name)
                };
                if file.mime_type == FOLDER_MIME {
                    let child = format!("{key}/");
                    if child.starts_with(prefix) || prefix.starts_with(&child) {
                        pending.push((file.id.clone(), key));
                    }
                } else if key.starts_with(prefix) && validate_key(&key).is_ok() {
                    out.push(file.meta(key));
                }
            }
        }
        Ok(())
    }
}

fn split(key: &str) -> (Vec<&str>, &str) {
    let mut parts: Vec<&str> = key.split('/').collect();
    let name = parts.pop().unwrap_or_default();
    (parts, name)
}

async fn refused(context: &str, resp: Response) -> Error {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| body.chars().take(300).collect());
    match status {
        StatusCode::NOT_FOUND => Error::NotFound {
            key: context.to_string(),
        },
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Error::Unauthorized { message },
        _ => Error::Backend {
            status: Some(status.as_u16()),
            message,
        },
    }
}

async fn read_json<T: serde::de::DeserializeOwned>(context: &str, resp: Response) -> Result<T> {
    if !resp.status().is_success() {
        return Err(refused(context, resp).await);
    }
    resp.json().await.map_err(|e| Error::Backend {
        status: None,
        message: e.to_string(),
    })
}

impl ObjectStore for DriveStore {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            conditional_put: false,
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
            if mode != PutMode::Overwrite {
                return Err(Error::Unsupported {
                    operation: "conditional put",
                });
            }
            let (folder, existing) = self.find(key, true).await?;
            let folder = folder.unwrap_or_default();
            let (_, name) = split(key);
            let metadata = match &existing {
                Some(_) => serde_json::json!({ "name": name }),
                None => serde_json::json!({ "name": name, "parents": [folder] }),
            };
            let suffix: u128 = rand::thread_rng().r#gen();
            let boundary = format!("objstore-upload-{suffix:032x}");
            let mut body = Vec::with_capacity(bytes.len() + 512);
            body.extend_from_slice(
                format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n")
                    .as_bytes(),
            );
            body.extend_from_slice(metadata.to_string().as_bytes());
            body.extend_from_slice(
                format!("\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n")
                    .as_bytes(),
            );
            body.extend_from_slice(&bytes);
            body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
            let params = [("uploadType", "multipart"), ("fields", FILE_FIELDS)];
            let request = match &existing {
                Some(file) => self.http.patch(format!(
                    "{}/{}",
                    self.upload_url(),
                    utf8_percent_encode(&file.id, NON_ALPHANUMERIC)
                )),
                None => self.http.post(self.upload_url()),
            };
            let resp = self
                .send(
                    request
                        .query(&params)
                        .header(
                            reqwest::header::CONTENT_TYPE,
                            format!("multipart/related; boundary={boundary}"),
                        )
                        .body(body),
                )
                .await?;
            let file: File = read_json(key, resp).await?;
            Ok(file.meta(key.to_string()))
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Object>> {
        Box::pin(async move {
            validate_key(key)?;
            let Some(file) = self.find(key, false).await?.1 else {
                return Err(Error::NotFound {
                    key: key.to_string(),
                });
            };
            let url = format!(
                "{}/{}",
                self.files_url(),
                utf8_percent_encode(&file.id, NON_ALPHANUMERIC)
            );
            let resp = self
                .send(self.http.get(url).query(&[("alt", "media")]))
                .await?;
            if !resp.status().is_success() {
                return Err(refused(key, resp).await);
            }
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| Error::Transport {
                    message: e.to_string(),
                })?
                .to_vec();
            let meta = ObjectMeta {
                size: bytes.len() as u64,
                ..file.meta(key.to_string())
            };
            Ok(Object { meta, bytes })
        })
    }

    fn head<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<ObjectMeta>> {
        Box::pin(async move {
            validate_key(key)?;
            match self.find(key, false).await?.1 {
                Some(file) => Ok(file.meta(key.to_string())),
                None => Err(Error::NotFound {
                    key: key.to_string(),
                }),
            }
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            validate_key(key)?;
            let Some(file) = self.find(key, false).await?.1 else {
                return Ok(());
            };
            let url = format!(
                "{}/{}",
                self.files_url(),
                utf8_percent_encode(&file.id, NON_ALPHANUMERIC)
            );
            let resp = self
                .send(
                    self.http
                        .patch(url)
                        .query(&[("fields", "id")])
                        .json(&serde_json::json!({ "trashed": true })),
                )
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
        _token: Option<String>,
    ) -> BoxFuture<'a, Result<ListPage>> {
        Box::pin(async move {
            validate_prefix(prefix)?;
            let base = prefix.rfind('/').map(|at| &prefix[..at]).unwrap_or("");
            let dirs: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
            let mut objects = Vec::new();
            if let Some(folder) = self.folder(&dirs, false).await? {
                self.walk(folder, base.to_string(), prefix, &mut objects)
                    .await?;
            }
            objects.sort_by(|a, b| a.key.cmp(&b.key));
            Ok(ListPage {
                objects,
                next: None,
            })
        })
    }
}
