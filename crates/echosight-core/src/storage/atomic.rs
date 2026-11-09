use super::{
    json_format,
    queue::{Ticket, resolve},
};
use serde_json::Value;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

/// Fault/ordering seams run immediately before real filesystem operations.
/// Hooks must not recursively call storage for the same path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Read,
    CreateDir,
    CreateTemp,
    Write,
    Sync,
    Close,
    Replace,
    Cleanup,
    Backup,
    Exists,
    CreateOnly,
}
pub trait StorageHooks: Send + Sync {
    fn before(&self, _stage: Stage, _path: &Path) -> io::Result<()> {
        Ok(())
    }
    /// Called after reserving a FIFO ticket, before waiting (non-failing).
    fn enqueued(&self, _path: &Path) {}
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}
struct SystemHooks;
impl StorageHooks for SystemHooks {}

#[derive(Clone)]
pub struct AtomicWriter {
    pub(super) hooks: Arc<dyn StorageHooks>,
}
impl Default for AtomicWriter {
    fn default() -> Self {
        Self::with_hooks(Arc::new(SystemHooks))
    }
}
impl AtomicWriter {
    pub fn with_hooks(hooks: Arc<dyn StorageHooks>) -> Self {
        Self { hooks }
    }

    /// Blocking, process-wide FIFO serialization by resolved path. Not a
    /// cross-process lock; symlink/case aliases are not canonicalized.
    pub fn write(&self, path: &Path, value: &Value) -> io::Result<()> {
        let path = resolve(path)?;
        let _ticket = self.lock(&path);
        self.write_locked(&path, value, false).map(|_| ())
    }

    pub(super) fn lock(&self, path: &Path) -> Ticket {
        let ticket = Ticket::new(path);
        self.hooks.enqueued(path);
        ticket.wait();
        ticket
    }

    pub(super) fn before(&self, stage: Stage, path: &Path) -> io::Result<()> {
        self.hooks.before(stage, path)
    }

    pub(super) fn write_locked(
        &self,
        path: &Path,
        value: &Value,
        create_only: bool,
    ) -> io::Result<bool> {
        let bytes = json_format(value).map_err(io::Error::other)?;
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("target has no parent"))?;
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::other("target has no filename"))?;
        self.before(Stage::CreateDir, parent)?;
        fs::create_dir_all(parent)?;
        let mut temp_name = std::ffi::OsString::from(".");
        temp_name.push(name);
        temp_name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let temp: PathBuf = parent.join(temp_name);
        self.before(Stage::CreateTemp, &temp)?;
        // Do not clean up on exclusive-create failure: that file is not ours.
        let mut handle = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let result = (|| {
            self.before(Stage::Write, &temp)?;
            handle.write_all(bytes.as_bytes())?;
            self.before(Stage::Sync, &temp)?;
            handle.sync_all()?;
            self.before(Stage::Close, &temp)?;
            Ok::<_, io::Error>(())
        })();
        // Rust File drop has no close-error return; sync_all above is mandatory.
        drop(handle);
        let result = result.and_then(|()| {
            if create_only {
                self.before(Stage::CreateOnly, path)?;
                // Atomic publication without ever replacing an existing path,
                // including a path created by another process after our check.
                match fs::hard_link(&temp, path) {
                    Ok(()) => Ok(true),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(false),
                    Err(error) => Err(error),
                }
            } else {
                self.before(Stage::Replace, path)?;
                // std::fs::rename replaces on Windows too (MoveFileExW with
                // MOVEFILE_REPLACE_EXISTING); never delete the destination first.
                fs::rename(&temp, path)?;
                Ok(true)
            }
        });
        if result.is_err() || create_only {
            // Keep the decisive write/publication result, even if cleanup fails.
            let _ = self
                .before(Stage::Cleanup, &temp)
                .and_then(|()| fs::remove_file(&temp));
        }
        result
    }
}
