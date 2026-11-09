use super::{FileKind, Stage, Storage, StorageError, parse_json};
use serde_json::Value;
use std::{fs, io, path::Path};

impl Storage {
    /// Create missing files only. Settings are supplied by the settings layer;
    /// bundled settings.json is never read. Existing bytes are not normalized.
    /// Hard-link publication fails closed on filesystems without hard-link support.
    pub fn bootstrap(&self, defaults: &Path, settings: &Value) -> Result<(), StorageError> {
        self.writer
            .before(Stage::CreateDir, self.paths.data_dir())?;
        fs::create_dir_all(self.paths.data_dir())?;
        for kind in [FileKind::Tasks, FileKind::Templates, FileKind::Settings] {
            let path = self.paths.file(kind);
            let _ticket = self.writer.lock(&path);
            self.writer.before(Stage::Exists, &path)?;
            match fs::symlink_metadata(&path) {
                Ok(_) => continue,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            let value = if kind == FileKind::Settings {
                settings.clone()
            } else {
                let source = defaults.join(kind.filename());
                let read = self
                    .writer
                    .before(Stage::Read, &source)
                    .and_then(|()| fs::read(&source))
                    .map_err(StorageError::from)
                    .and_then(|bytes| {
                        parse_json(&String::from_utf8_lossy(&bytes)).map_err(StorageError::from)
                    });
                read.unwrap_or_else(|error| {
                    log::warn!(
                        "Could not read bundled runtime default {}; using empty fallback: {error}",
                        source.display()
                    );
                    kind.fallback()
                })
            };
            self.writer.write_locked(&path, &value, true)?;
        }
        Ok(())
    }
}
