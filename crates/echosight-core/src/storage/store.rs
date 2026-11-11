use super::{
    AtomicWriter, ParseError, Stage, StorageHooks, parse_json,
    queue::{resolve, unpoison},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Tasks,
    Templates,
    Settings,
}
impl FileKind {
    pub fn filename(self) -> &'static str {
        match self {
            Self::Tasks => "current_tasks.json",
            Self::Templates => "templates.json",
            Self::Settings => "settings.json",
        }
    }
    pub(super) fn fallback(self) -> Value {
        match self {
            Self::Tasks => json!({"tasks": [], "currentTemplate": null, "snapshots": []}),
            Self::Templates => json!([]),
            Self::Settings => Value::Null,
        }
    }
}

/// Where a loaded value came from. Never persisted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadOrigin {
    /// The file did not exist; the fallback was returned and nothing was written.
    Missing,
    /// The file parsed successfully (including an explicit JSON `null`).
    Existing,
    /// Corrupt bytes were backed up and the fallback was written.
    Recovered,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedValue {
    pub value: Value,
    pub origin: LoadOrigin,
}

/// An explicitly authorized data directory, not an application-identity guess.
/// Construction does no filesystem IO beyond resolving the current directory.
#[derive(Clone, Debug)]
pub struct DataPaths {
    pub(super) data: PathBuf,
}
impl DataPaths {
    pub fn new(data: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            data: resolve(data.as_ref())?,
        })
    }
    pub fn file(&self, kind: FileKind) -> PathBuf {
        self.data.join(kind.filename())
    }
    pub fn themes(&self) -> PathBuf {
        self.data.join("themes")
    }
    pub fn data_dir(&self) -> &Path {
        &self.data
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("Save blocked after load failure: {0}")]
    Blocked(String),
}

/// Share one Storage (or its clones) per runtime, like one IPC registration.
/// Loads, recovery and saves serialize on the same process-wide path queue;
/// load-failure blocks are shared by clones, but not independent registrations.
#[derive(Clone)]
pub struct Storage {
    pub(super) paths: DataPaths,
    pub(super) writer: AtomicWriter,
    blocked: Arc<Mutex<HashMap<PathBuf, String>>>,
}
impl Storage {
    pub fn new(paths: DataPaths) -> Self {
        Self {
            paths,
            writer: AtomicWriter::default(),
            blocked: Arc::default(),
        }
    }
    pub fn with_hooks(paths: DataPaths, hooks: Arc<dyn StorageHooks>) -> Self {
        Self {
            paths,
            writer: AtomicWriter::with_hooks(hooks),
            blocked: Arc::default(),
        }
    }

    /// IPC-compatible return shape: only recovered tasks expose corruptBackupPath.
    /// A missing file returns the fallback without creating either file or parent.
    pub fn load(&self, kind: FileKind) -> Result<Value, StorageError> {
        self.load_with_origin(kind).map(|loaded| loaded.value)
    }

    /// Same single locked load/recovery/block flow as `load`, also reporting
    /// whether the value was missing, existing, or recovered from corruption.
    pub fn load_with_origin(&self, kind: FileKind) -> Result<LoadedValue, StorageError> {
        let path = self.paths.file(kind);
        let _ticket = self.writer.lock(&path);
        let result = self.load_locked(&path, kind);
        match &result {
            Ok(_) => {
                unpoison(&self.blocked).remove(&path);
            }
            Err(error) => {
                unpoison(&self.blocked).insert(path, error.to_string());
            }
        }
        result
    }

    fn load_locked(&self, path: &Path, kind: FileKind) -> Result<LoadedValue, StorageError> {
        let bytes = match self
            .writer
            .before(Stage::Read, path)
            .and_then(|()| fs::read(path))
        {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(LoadedValue {
                    value: kind.fallback(),
                    origin: LoadOrigin::Missing,
                });
            }
            Err(error) => return Err(error.into()),
        };
        // Node Buffer.toString('utf8') replaces invalid UTF-8 instead of failing.
        match parse_json(&String::from_utf8_lossy(&bytes)) {
            Ok(value) => Ok(LoadedValue {
                value,
                origin: LoadOrigin::Existing,
            }),
            Err(error @ ParseError::Unsupported(_)) => Err(error.into()),
            Err(ParseError::Invalid(_)) => {
                let stamp = self
                    .writer
                    .hooks
                    .now()
                    .format("%Y-%m-%dT%H-%M-%S-%3fZ")
                    .to_string();
                let mut backup = path.as_os_str().to_os_string();
                backup.push(format!(".corrupt-{stamp}"));
                let backup = PathBuf::from(backup);
                self.writer.before(Stage::Backup, &backup)?;
                fs::copy(path, &backup)?;
                // Never publish the fallback before the original bytes are copied.
                let mut value = kind.fallback();
                self.writer.write_locked(path, &value, false)?;
                log::error!(
                    "Preserved corrupt data file: {} -> {}",
                    path.display(),
                    backup.display()
                );
                if kind == FileKind::Tasks {
                    value["corruptBackupPath"] =
                        Value::String(backup.to_string_lossy().into_owned());
                }
                Ok(LoadedValue {
                    value,
                    origin: LoadOrigin::Recovered,
                })
            }
        }
    }

    pub fn save(&self, kind: FileKind, value: &Value) -> Result<(), StorageError> {
        let path = self.paths.file(kind);
        let _ticket = self.writer.lock(&path);
        if let Some(error) = unpoison(&self.blocked).get(&path) {
            return Err(StorageError::Blocked(error.clone()));
        }
        self.writer.write_locked(&path, value, false)?;
        Ok(())
    }
}
