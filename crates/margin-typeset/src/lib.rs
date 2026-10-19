pub mod fonts;
mod pdf;

use serde::{Deserialize, Serialize};

pub use pdf::compile;

/// An image the document references, already read by the caller. A failed read becomes a warning
/// and a placeholder of the same format, so one missing picture never sinks the export.
pub struct Image {
    pub path: String,
    pub bytes: Result<Vec<u8>, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Warning {
    pub kind: String,
    pub message: String,
    pub count: u32,
}
