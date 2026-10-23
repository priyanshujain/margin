//! Record sync for the Margin apps: every device appends its own encrypted journal segments and
//! manifest to a shared object store, takes in everyone else's, and merges last writer wins per
//! `(kind, id, field)`. Periodic snapshots make the same store the backup.

mod clock;
mod engine;
mod error;
pub mod memory;
mod record;
mod replica;
mod storage;
mod suite;
pub mod wire;

pub use clock::{Hlc, now_ms};
pub use engine::{DeviceInfo, Policy, Report, Space, last_synced_ms};
pub use error::SyncError;
pub use margin_crypt::{Key, Phrase, RootKey};
pub use record::{Record, Slot, Winners};
pub use replica::{MemoryReplica, Replica};
pub use storage::{DRIVE_ROOT, Storage, TokenProvider};
pub use suite::Suite;
