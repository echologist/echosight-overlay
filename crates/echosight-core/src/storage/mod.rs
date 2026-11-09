//! Bounded Electron-compatible JSON persistence. No domain models or app-directory
//! discovery live here; callers must supply the authorized data directory.
mod atomic;
mod bootstrap;
mod store;
pub use store::{DataPaths, FileKind, Storage, StorageError};
mod json;
mod queue;
pub use atomic::{AtomicWriter, Stage, StorageHooks};
pub mod json_format;
pub use json::{ParseError, parse_json};
pub use json_format::json_format;
