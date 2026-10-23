use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    Store(objstore::Error),
    /// An object would not decrypt: the recovery phrase is not the one this store was made with.
    WrongKey {
        key: String,
    },
    Corrupt {
        key: String,
        detail: String,
    },
    /// Two installations are publishing under one device id, which happens when a data directory
    /// is copied to another machine.
    DeviceClash {
        device: String,
    },
    Replica(String),
    Config(String),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::Store(e) => write!(f, "{e}"),
            SyncError::WrongKey { key } => write!(
                f,
                "{key} could not be decrypted. The recovery phrase is not the one this storage was set up with, or the file was changed."
            ),
            SyncError::Corrupt { key, detail } => write!(f, "{key} is not readable: {detail}"),
            SyncError::DeviceClash { device } => write!(
                f,
                "another installation is syncing as device {device}. This usually means a data folder was copied between machines."
            ),
            SyncError::Replica(e) => write!(f, "{e}"),
            SyncError::Config(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<objstore::Error> for SyncError {
    fn from(e: objstore::Error) -> SyncError {
        SyncError::Store(e)
    }
}

impl From<SyncError> for String {
    fn from(e: SyncError) -> String {
        e.to_string()
    }
}
