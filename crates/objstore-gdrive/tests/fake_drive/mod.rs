use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};

pub const TOKEN: &str = "token-ok";
const FOLDER: &str = "application/vnd.google-apps.folder";
const PAGE: usize = 3;

#[derive(Clone, Debug)]
pub struct FakeFile {
    pub id: String,
    pub name: String,
    pub mime: String,
    pub parents: Vec<String>,
    pub bytes: Vec<u8>,
    pub version: u64,
    pub trashed: bool,
}

#[derive(Default)]
pub struct State {
    pub files: Vec<FakeFile>,
    next: u64,
}

impl State {
    pub fn add(&mut self, name: &str, mime: &str, parent: Option<&str>, bytes: &[u8]) -> String {
        self.next += 1;
        let id = format!("id{}", self.next);
        self.files.push(FakeFile {
            id: id.clone(),
            name: name.to_string(),
            mime: mime.to_string(),
            parents: parent.map(|p| vec![p.to_string()]).unwrap_or_default(),
            bytes: bytes.to_vec(),
            version: 1,
            trashed: false,
        });
        id
    }

    pub fn folder(&mut self, name: &str, parent: Option<&str>) -> String {
        self.add(name, FOLDER, parent, b"")
    }
}

pub async fn start() -> (String, Arc<Mutex<State>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let state = Arc::new(Mutex::new(State::default()));
    let shared = state.clone();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let state = shared.clone();
            tokio::spawn(async move {
                let service = hyper::service::service_fn(move |req| handle(state.clone(), req));
                let _ = hyper::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    });
    (base, state)
}

fn reply(status: StatusCode, body: Vec<u8>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

fn json_reply(value: Value) -> Response<Full<Bytes>> {
    reply(StatusCode::OK, value.to_string().into_bytes())
}

fn describe(file: &FakeFile) -> Value {
    json!({
        "id": file.id, "name": file.name, "mimeType": file.mime,
        "size": file.bytes.len().to_string(), "version": file.version.to_string(),
        "modifiedTime": "2026-10-20T10:00:00.000Z",
    })
}

async fn handle(
    state: Arc<Mutex<State>>,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let authorised = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        == Some(&format!("Bearer {TOKEN}"));
    if !authorised {
        return Ok(reply(
            StatusCode::UNAUTHORIZED,
            br#"{"error":{"message":"Invalid Credentials"}}"#.to_vec(),
        ));
    }
    let method = req.method().clone();
    let uri = req.uri().clone();
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let query: Vec<(String, String)> =
        url::form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
            .into_owned()
            .collect();
    let param = |name: &str| {
        query
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    let body = req.into_body().collect().await.unwrap().to_bytes().to_vec();
    let path = uri.path().to_string();
    let mut state = state.lock().unwrap();

    let response = match (method.as_str(), path.as_str()) {
        ("GET", "/drive/v3/files") => {
            let q = param("q").unwrap_or_default();
            let matching: Vec<Value> = state
                .files
                .iter()
                .filter(|f| matches(&q, f))
                .map(describe)
                .collect();
            let offset: usize = param("pageToken").and_then(|t| t.parse().ok()).unwrap_or(0);
            let size = param("pageSize")
                .and_then(|s| s.parse().ok())
                .unwrap_or(PAGE)
                .min(PAGE);
            let page: Vec<Value> = matching.iter().skip(offset).take(size).cloned().collect();
            let mut out = json!({ "files": page });
            if offset + size < matching.len() {
                out["nextPageToken"] = json!((offset + size).to_string());
            }
            json_reply(out)
        }
        ("POST", "/drive/v3/files") => {
            let meta: Value = serde_json::from_slice(&body).unwrap();
            let parent = meta["parents"][0].as_str().map(str::to_string);
            let id = state.add(
                meta["name"].as_str().unwrap(),
                meta["mimeType"].as_str().unwrap(),
                parent.as_deref(),
                b"",
            );
            json_reply(json!({ "id": id }))
        }
        ("POST", "/upload/drive/v3/files") => {
            let (meta, bytes) = multipart(&content_type, &body);
            let parent = meta["parents"][0].as_str().map(str::to_string);
            let id = state.add(
                meta["name"].as_str().unwrap(),
                "application/octet-stream",
                parent.as_deref(),
                &bytes,
            );
            let file = state.files.iter().find(|f| f.id == id).unwrap();
            json_reply(describe(file))
        }
        ("PATCH", p) if p.starts_with("/upload/drive/v3/files/") => {
            let id = &p["/upload/drive/v3/files/".len()..];
            let (_, bytes) = multipart(&content_type, &body);
            match state.files.iter_mut().find(|f| f.id == id) {
                Some(file) => {
                    file.bytes = bytes;
                    file.version += 1;
                    json_reply(describe(file))
                }
                None => reply(StatusCode::NOT_FOUND, b"{}".to_vec()),
            }
        }
        ("GET", p)
            if p.starts_with("/drive/v3/files/") && param("alt").as_deref() == Some("media") =>
        {
            let id = &p["/drive/v3/files/".len()..];
            match state.files.iter().find(|f| f.id == id) {
                Some(file) => reply(StatusCode::OK, file.bytes.clone()),
                None => reply(StatusCode::NOT_FOUND, b"{}".to_vec()),
            }
        }
        ("PATCH", p) if p.starts_with("/drive/v3/files/") => {
            let id = &p["/drive/v3/files/".len()..];
            let meta: Value = serde_json::from_slice(&body).unwrap();
            match state.files.iter_mut().find(|f| f.id == id) {
                Some(file) => {
                    file.trashed = meta["trashed"].as_bool().unwrap_or(file.trashed);
                    json_reply(json!({ "id": file.id }))
                }
                None => reply(StatusCode::NOT_FOUND, b"{}".to_vec()),
            }
        }
        _ => reply(
            StatusCode::BAD_REQUEST,
            format!("unexpected {method} {path}").into_bytes(),
        ),
    };
    Ok(response)
}

fn quoted(text: &str) -> (String, &str) {
    let mut out = String::new();
    let mut chars = text.char_indices().skip(1);
    while let Some((at, c)) = chars.next() {
        match c {
            '\\' => out.push(chars.next().unwrap().1),
            '\'' => return (out, &text[at + 1..]),
            _ => out.push(c),
        }
    }
    panic!("unterminated string in {text}");
}

fn matches(q: &str, file: &FakeFile) -> bool {
    let mut rest = q.trim();
    loop {
        let ok = if let Some(after) = rest.strip_prefix("name = ") {
            let (value, tail) = quoted(after);
            rest = tail;
            file.name == value
        } else if let Some(after) = rest.strip_prefix("mimeType = ") {
            let (value, tail) = quoted(after);
            rest = tail;
            file.mime == value
        } else if let Some(after) = rest.strip_prefix("mimeType != ") {
            let (value, tail) = quoted(after);
            rest = tail;
            file.mime != value
        } else if let Some(tail) = rest.strip_prefix("trashed = false") {
            rest = tail;
            !file.trashed
        } else if rest.starts_with('\'') {
            let (value, tail) = quoted(rest);
            rest = tail.strip_prefix(" in parents").expect("in parents clause");
            file.parents.contains(&value)
        } else {
            panic!("unsupported query {rest}");
        };
        if !ok {
            return false;
        }
        match rest.trim().strip_prefix("and ") {
            Some(next) => rest = next,
            None if rest.trim().is_empty() => return true,
            None => panic!("unsupported query tail {rest}"),
        }
    }
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> usize {
    from + haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("multipart delimiter")
}

fn multipart(content_type: &str, body: &[u8]) -> (Value, Vec<u8>) {
    let boundary = content_type.split("boundary=").nth(1).expect("boundary");
    let delimiter = format!("--{boundary}");
    let first = find(body, delimiter.as_bytes(), 0);
    let meta_start = find(body, b"\r\n\r\n", first) + 4;
    let second = find(body, format!("\r\n{delimiter}").as_bytes(), meta_start);
    let meta: Value = serde_json::from_slice(&body[meta_start..second]).unwrap();
    let data_start = find(body, b"\r\n\r\n", second + 2) + 4;
    let end = find(body, format!("\r\n{delimiter}--").as_bytes(), data_start);
    (meta, body[data_start..end].to_vec())
}
