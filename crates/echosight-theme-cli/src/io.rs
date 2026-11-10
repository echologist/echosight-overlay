//! Explicit, bounded filesystem adapter. No ambient app paths or bundled sync.
use echosight_core::theme::ThemeRoot;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const TEXT_LIMIT: usize = 1024 * 1024;
pub const ENTRY_LIMIT: usize = 1024;
pub const ASSET_LIMIT: usize = 32 * 1024 * 1024;
pub const TOTAL_LIMIT: usize = 64 * 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);
pub fn error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

/// Remember both the user-authorized alias and its canonical identity. Recheck
/// before each operation (ThemeRoot::existing alone permits contained retargets).
#[derive(Debug)]
pub struct Root {
    theme: ThemeRoot,
    canonical: PathBuf,
    #[cfg(unix)]
    identity: (u64, u64),
}
impl Root {
    pub fn new(path: &Path) -> io::Result<Self> {
        let theme = ThemeRoot::new(path)?;
        let canonical = fs::canonicalize(theme.path())?;
        Ok(Self {
            #[cfg(unix)]
            identity: identity(&fs::metadata(&canonical)?),
            canonical,
            theme,
        })
    }
    pub fn path(&self) -> &Path {
        self.theme.path()
    }
    pub fn canonical(&self) -> &Path {
        &self.canonical
    }
    pub fn check(&self) -> io::Result<()> {
        if fs::canonicalize(self.path())? != self.canonical {
            return Err(error("authorized root was retargeted"));
        }
        #[cfg(unix)]
        if identity(&fs::metadata(self.path())?) != self.identity {
            return Err(error("authorized root was replaced"));
        }
        Ok(())
    }
    pub fn existing(&self, path: &Path) -> io::Result<PathBuf> {
        self.check()?;
        self.theme.existing(path)
    }
    pub fn read(&self, path: &Path, limit: usize) -> io::Result<Vec<u8>> {
        let path = self.existing(path)?;
        if !fs::metadata(&path)?.is_file() {
            return Err(error("input must be a regular file"));
        }
        let file = fs::File::open(&path)?;
        if !file.metadata()?.is_file() {
            return Err(error("input must be a regular file"));
        }
        if file.metadata()?.len() > limit as u64 {
            return Err(error("file byte budget exceeded"));
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(error("file byte budget exceeded"));
        }
        self.check()?;
        Ok(bytes)
    }
    pub fn relative_read(&self, name: &str, limit: usize) -> io::Result<Vec<u8>> {
        relative(name)?;
        self.read(&self.path().join(name), limit)
    }
}
#[cfg(unix)]
fn identity(metadata: &fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (metadata.dev(), metadata.ino())
}

pub fn relative(name: &str) -> io::Result<()> {
    if name.is_empty()
        || name.contains(['\\', ':', '\0'])
        || name.starts_with('/')
        || name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || Path::new(name)
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(error(format!("unsafe relative theme path: {name:?}")));
    }
    Ok(())
}
pub fn entries(root: &Root) -> io::Result<Vec<fs::DirEntry>> {
    root.check()?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(root.path())? {
        if entries.len() == ENTRY_LIMIT {
            return Err(error("directory entry budget exceeded (1024)"));
        }
        entries.push(entry?);
    }
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}
pub struct Source {
    pub root: Root,
    pub document: serde_json::Value,
    pub protected: PathBuf,
    pub folder: bool,
}
impl Source {
    pub fn open(path: &Path) -> io::Result<Self> {
        let path = std::path::absolute(path)?;
        // Explicit input aliases are allowed, but the chosen folder bounds all children.
        let (root, file) = if fs::metadata(&path)?.is_dir() {
            let root = Root::new(&path)?;
            let file = root.path().join("theme.json");
            (root, file)
        } else {
            let root = Root::new(path.parent().ok_or_else(|| error("input has no parent"))?)?;
            let file = root.path().join(
                path.file_name()
                    .ok_or_else(|| error("input has no filename"))?,
            );
            (root, file)
        };
        let bytes = root.read(&file, TEXT_LIMIT)?;
        let document = serde_json::from_slice(&bytes).map_err(error_json)?;
        let folder = path.is_dir() || path.file_name().is_some_and(|name| name == "theme.json");
        let protected = if folder {
            root.canonical().to_path_buf()
        } else {
            root.existing(&file)?
        };
        Ok(Self {
            root,
            document,
            protected,
            folder,
        })
    }
}
fn error_json(e: serde_json::Error) -> io::Error {
    error(format!("invalid JSON: {e}"))
}

/// Existing parents only: never create an inferred ancestor hierarchy.
pub struct Destination {
    parent: Root,
    path: PathBuf,
}
impl Destination {
    pub fn new(path: &Path, source: &Path) -> io::Result<Self> {
        let path = std::path::absolute(path)?;
        if path.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(error("output path must not contain .."));
        }
        let parent = Root::new(path.parent().ok_or_else(|| error("output has no parent"))?)?;
        let name = path
            .file_name()
            .ok_or_else(|| error("output has no filename"))?;
        let resolved = parent.canonical().join(name);
        if resolved.starts_with(source) || source.starts_with(&resolved) {
            return Err(error("output overlaps authorized source root"));
        }
        let result = Self {
            path: parent.path().join(name),
            parent,
        };
        result.check()?;
        Ok(result)
    }
    fn check(&self) -> io::Result<()> {
        self.parent.check()?;
        match fs::symlink_metadata(&self.path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
            Ok(_) => Err(error(
                "output already exists (including symlinks); refusing to overwrite",
            )),
        }
    }
    pub fn publish(&self, payload: &BTreeMap<String, Vec<u8>>) -> io::Result<()> {
        self.check()?;
        let stage = loop {
            self.parent.check()?;
            let name = format!(
                ".echosight-theme-{}-{}.tmp",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            match OwnedDirectory::create(&self.parent.path().join(name)) {
                Ok(stage) => break stage,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        };
        let mut stage = stage;
        for (name, bytes) in payload {
            self.parent.check()?;
            stage.write(name, bytes)?;
        }
        self.check()?;
        let mut output = OwnedDirectory::create(&self.path)?;
        // Manifest last is the commit marker. Hard links atomically refuse an
        // existing target, unlike rename. No partial manifest is ever written.
        for name in payload
            .keys()
            .filter(|name| name.as_str() != "theme.json")
            .chain(std::iter::once(&"theme.json".to_owned()))
        {
            self.parent.check()?;
            stage.root.check()?;
            output.link(name, &stage.root.path().join(name))?;
        }
        output.keep = true;
        Ok(())
    }
}
/// Ownership ledger: never recursively delete a directory supplied by a caller.
struct OwnedDirectory {
    root: Root,
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
    keep: bool,
}
impl OwnedDirectory {
    fn create(path: &Path) -> io::Result<Self> {
        let builder = fs::DirBuilder::new();
        #[cfg(unix)]
        let builder = {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = builder;
            builder.mode(0o700);
            builder
        };
        builder.create(path)?;
        Ok(Self {
            root: Root::new(path)?,
            files: vec![],
            dirs: vec![],
            keep: false,
        })
    }
    fn target(&mut self, name: &str) -> io::Result<PathBuf> {
        relative(name)?;
        self.root.check()?;
        let target = self.root.path().join(name);
        let parent = Path::new(name).parent().unwrap();
        let mut current = self.root.path().to_path_buf();
        for part in parent.components() {
            current.push(part);
            match fs::create_dir(&current) {
                Ok(()) => self.dirs.push(current.clone()),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    if !self.dirs.contains(&current)
                        || fs::symlink_metadata(&current)?.file_type().is_symlink()
                    {
                        return Err(error("unowned output directory"));
                    }
                }
                Err(e) => return Err(e),
            }
            self.root.existing(&current)?;
        }
        Ok(target)
    }
    fn write(&mut self, name: &str, bytes: &[u8]) -> io::Result<()> {
        let path = self.target(name)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        self.files.push(path);
        file.write_all(bytes)?;
        file.sync_all()
    }
    fn link(&mut self, name: &str, from: &Path) -> io::Result<()> {
        let path = self.target(name)?;
        fs::hard_link(from, &path)?;
        self.files.push(path);
        Ok(())
    }
}
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        if self.keep || self.root.check().is_err() {
            return;
        }
        for path in self.files.iter().rev() {
            if self.root.existing(path).is_ok() {
                let _ = fs::remove_file(path);
            }
        }
        for path in self.dirs.iter().rev() {
            if self.root.existing(path).is_ok() {
                let _ = fs::remove_dir(path);
            }
        }
        let _ = fs::remove_dir(self.root.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Sandbox(PathBuf);
    impl Sandbox {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "theme-tools-io-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn payload() -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([
            ("theme.json".into(), b"{}".to_vec()),
            ("nested/a.svg".into(), b"<svg/>".to_vec()),
            ("CONVERSION-REPORT.md".into(), b"report".to_vec()),
        ])
    }
    #[test]
    fn publication_refuses_late_existing_destination_and_cleans_staging_failure() {
        let s = Sandbox::new();
        let out = s.0.join("out");
        let dest = Destination::new(&out, &s.0.join("source")).unwrap();
        fs::create_dir(&out).unwrap();
        assert!(dest.publish(&payload()).is_err());
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
        fs::remove_dir(&out).unwrap();
        let mut bad = payload();
        bad.insert("a".into(), vec![]);
        bad.insert("a/b.svg".into(), vec![]);
        assert!(dest.publish(&bad).is_err());
        assert_eq!(fs::read_dir(&s.0).unwrap().count(), 0);
        dest.publish(&payload()).unwrap();
        assert_eq!(fs::read(out.join("nested/a.svg")).unwrap(), b"<svg/>");
        assert_eq!(fs::read_dir(&s.0).unwrap().count(), 1);
    }
    #[test]
    fn cleanup_removes_only_owned_files_not_foreign_children() {
        let s = Sandbox::new();
        let out = s.0.join("out");
        let mut owned = OwnedDirectory::create(&out).unwrap();
        owned.write("ours.svg", b"ours").unwrap();
        fs::write(out.join("foreign"), b"preserve").unwrap();
        drop(owned);
        assert!(!out.join("ours.svg").exists());
        assert_eq!(fs::read(out.join("foreign")).unwrap(), b"preserve");
        assert!(!out.join("theme.json").exists());
    }
    #[cfg(unix)]
    #[test]
    fn source_and_destination_roots_reject_sequential_retarget_and_replacement() {
        use std::os::unix::fs::symlink;
        let s = Sandbox::new();
        for name in ["real", "real/child", "outside"] {
            fs::create_dir(s.0.join(name)).unwrap();
        }
        symlink("real", s.0.join("alias")).unwrap();
        let source = Root::new(&s.0.join("alias")).unwrap();
        fs::write(s.0.join("real/input"), b"real").unwrap();
        assert_eq!(source.relative_read("input", 10).unwrap(), b"real");
        let dest = Destination::new(&s.0.join("alias/out"), &s.0.join("source")).unwrap();
        fs::remove_file(s.0.join("alias")).unwrap();
        symlink("real/child", s.0.join("alias")).unwrap();
        assert!(source.relative_read("input", 10).is_err());
        assert!(dest.publish(&payload()).is_err());
        assert_eq!(fs::read_dir(s.0.join("real/child")).unwrap().count(), 0);
        fs::remove_file(s.0.join("alias")).unwrap();
        symlink("outside", s.0.join("alias")).unwrap();
        assert!(dest.publish(&payload()).is_err());
        assert_eq!(fs::read_dir(s.0.join("outside")).unwrap().count(), 0);
        let direct = Destination::new(&s.0.join("real/out"), &s.0.join("source")).unwrap();
        fs::rename(s.0.join("real"), s.0.join("old")).unwrap();
        fs::create_dir(s.0.join("real")).unwrap();
        assert!(direct.publish(&payload()).is_err());
        assert_eq!(fs::read_dir(s.0.join("real")).unwrap().count(), 0);
    }
}
