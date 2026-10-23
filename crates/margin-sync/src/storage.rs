use objstore::{ObjectStore, Prefixed};
use serde::{Deserialize, Serialize};

pub use objstore_gdrive::TokenProvider;

use crate::error::SyncError;

/// The Drive folder every Margin app writes under, the one Mail's backups already live in.
pub const DRIVE_ROOT: &str = "margin";

/// Where the suite's sync store is. Secrets (`secret`, `password`) are kept out of the plain
/// config file by [`crate::Suite`] and sealed beside the phrase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Storage {
    Folder {
        path: String,
    },
    S3 {
        endpoint: String,
        #[serde(default = "auto_region")]
        region: String,
        bucket: String,
        access_key: String,
        #[serde(default, skip_serializing)]
        secret: String,
        #[serde(default)]
        prefix: Option<String>,
        #[serde(default)]
        allow_http: bool,
    },
    WebDav {
        url: String,
        #[serde(default)]
        username: Option<String>,
        #[serde(default, skip_serializing)]
        password: String,
        #[serde(default)]
        conditional_put: bool,
        #[serde(default)]
        allow_http: bool,
    },
    /// The Drive of one Google account, named by address. The app that owns that account's
    /// sign-in supplies its tokens.
    GoogleDrive {
        account: String,
    },
}

/// Plain HTTP is only accepted to this machine, where nothing crosses a network.
fn loopback(endpoint: &str) -> bool {
    let rest = endpoint.strip_prefix("http://").unwrap_or("");
    let host = rest.split(['/', ':']).next().unwrap_or("");
    matches!(host, "127.0.0.1" | "localhost") || rest.starts_with("[::1]")
}

fn auto_region() -> String {
    "auto".to_string()
}

impl Storage {
    pub fn kind(&self) -> &'static str {
        match self {
            Storage::Folder { .. } => "folder",
            Storage::S3 { .. } => "s3",
            Storage::WebDav { .. } => "web-dav",
            Storage::GoogleDrive { .. } => "google-drive",
        }
    }

    pub fn secret(&self) -> Option<&str> {
        match self {
            Storage::S3 { secret, .. } => Some(secret),
            Storage::WebDav { password, .. } => Some(password),
            _ => None,
        }
    }

    pub fn set_secret(&mut self, value: String) {
        match self {
            Storage::S3 { secret, .. } => *secret = value,
            Storage::WebDav { password, .. } => *password = value,
            _ => {}
        }
    }

    /// A short description for a settings screen, never including a secret.
    pub fn describe(&self) -> String {
        match self {
            Storage::Folder { path } => path.clone(),
            Storage::S3 {
                endpoint, bucket, ..
            } => format!("{bucket} at {endpoint}"),
            Storage::WebDav { url, .. } => url.clone(),
            Storage::GoogleDrive { account } => format!("Google Drive of {account}"),
        }
    }

    /// Which store this is, without any secret, for telling one store from another.
    pub fn identity(&self) -> String {
        match self {
            Storage::S3 {
                endpoint,
                bucket,
                prefix,
                ..
            } => format!("s3:{endpoint}/{bucket}/{}", prefix.as_deref().unwrap_or("")),
            Storage::WebDav { url, .. } => format!("web-dav:{url}"),
            Storage::Folder { path } => format!("folder:{path}"),
            Storage::GoogleDrive { account } => format!("google-drive:{account}"),
        }
    }

    pub fn open(
        &self,
        drive_token: Option<TokenProvider>,
    ) -> Result<Box<dyn ObjectStore>, SyncError> {
        let config = |e: objstore::Error| SyncError::Config(e.to_string());
        match self {
            Storage::Folder { path } => {
                if path.trim().is_empty() {
                    return Err(SyncError::Config("Choose a folder to sync through.".into()));
                }
                Ok(Box::new(objstore_fs::FsStore::new(path).map_err(config)?))
            }
            Storage::S3 {
                endpoint,
                region,
                bucket,
                access_key,
                secret,
                prefix,
                allow_http,
            } => {
                let mut c = objstore_s3::Config::new(
                    endpoint.clone(),
                    region.clone(),
                    bucket.clone(),
                    access_key.clone(),
                    secret.clone(),
                );
                c.allow_http = *allow_http || loopback(endpoint);
                let store = objstore_s3::S3Store::new(c).map_err(config)?;
                match prefix.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
                    Some(prefix) => Ok(Box::new(Prefixed::new(store, prefix).map_err(config)?)),
                    None => Ok(Box::new(store)),
                }
            }
            Storage::WebDav {
                url,
                username,
                password,
                conditional_put,
                allow_http,
            } => {
                let mut c = objstore_webdav::Config::new(url.clone());
                c.username = username.clone();
                c.password = (!password.is_empty()).then(|| password.clone());
                c.conditional_put = *conditional_put;
                c.allow_http = *allow_http || loopback(url);
                Ok(Box::new(
                    objstore_webdav::WebDavStore::new(c).map_err(config)?,
                ))
            }
            Storage::GoogleDrive { .. } => {
                let token = drive_token.ok_or_else(|| {
                    SyncError::Config("Google Drive needs a signed-in account.".into())
                })?;
                Ok(Box::new(
                    objstore_gdrive::DriveStore::new(
                        objstore_gdrive::Config::new([DRIVE_ROOT]),
                        token,
                    )
                    .map_err(config)?,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::loopback;

    #[test]
    fn only_loopback_endpoints_may_use_plain_http() {
        assert!(loopback("http://127.0.0.1:9100"));
        assert!(loopback("http://localhost/dav"));
        assert!(loopback("http://[::1]:9000"));
        assert!(!loopback("http://127.0.0.1.example.com"));
        assert!(!loopback("http://minio.lan:9000"));
        assert!(!loopback("https://127.0.0.1"));
    }
}
