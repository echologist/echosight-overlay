//! Filesystem compatibility delivery, not a CSS execution engine.
use super::{get_mime_type, helpers::js_trim, v1::model::Theme};
use base64::{Engine, engine::general_purpose::STANDARD};
use regex::Regex;
use serde_json::{Value, json};
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    sync::LazyLock,
};

/// An explicitly authorized existing root. No app-directory discovery or ambient
/// user directory access. Both lexical paths and canonical targets must remain
/// inside this root. Checks protect static trees, not concurrent symlink swaps.
#[derive(Debug, Clone)]
pub struct ThemeRoot {
    path: PathBuf,
}
impl ThemeRoot {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = fs::canonicalize(path)?;
        if !path.is_dir() {
            return Err(io::Error::other("theme root is not a directory"));
        }
        Ok(Self { path })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn existing(&self, path: &Path) -> io::Result<PathBuf> {
        lexical_inside(&self.path, path)?;
        let canonical = fs::canonicalize(path)?;
        if !canonical.starts_with(&self.path) {
            return Err(denied());
        }
        Ok(canonical)
    }
    /// Check every existing ancestor, including a dangling final symlink, before
    /// creating missing children. Used only for the explicitly supplied sync root.
    pub(super) fn destination(&self, path: &Path) -> io::Result<()> {
        lexical_inside(&self.path, path)?;
        let mut current = self.path.clone();
        for part in path
            .strip_prefix(&self.path)
            .map_err(|_| denied())?
            .components()
        {
            current.push(part);
            match fs::symlink_metadata(&current) {
                Ok(meta) => {
                    if meta.file_type().is_symlink() {
                        return Err(denied());
                    }
                    self.existing(&current)?;
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    fn theme_directory(&self, theme: &Theme) -> io::Result<Option<PathBuf>> {
        let directory = theme
            .get("folderPath")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                theme
                    .get("path")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .and_then(|s| Path::new(s).parent().map(Path::to_path_buf))
            });
        directory.map(|p| self.existing(&p)).transpose()
    }
    pub fn load_asset(&self, theme: Option<&Theme>, name: &Value) -> io::Result<Option<Value>> {
        let Some(theme) = theme else { return Ok(None) };
        let Some(name) = name.as_str() else {
            return Ok(None);
        };
        let Some(asset) = theme.get("assets").and_then(|v| v.get(name)) else {
            return Ok(None);
        };
        let Some(raw_path) = asset
            .get("path")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            return Ok(None);
        };
        let directory = match self.theme_directory(theme) {
            Ok(Some(p)) => p,
            _ => return Ok(None),
        };
        let path = Path::new(raw_path);
        if lexical_inside(&directory, path).is_err() {
            return Ok(None);
        }
        let canonical = match self.existing(path) {
            Ok(p) if p.starts_with(&directory) => p,
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => return Ok(None),
            Err(e) => return Err(e),
            _ => return Ok(None),
        };
        let bytes = fs::read(canonical)?;
        let is_text = asset.get("type").and_then(Value::as_str) == Some("css");
        let extension = asset.get("extension").and_then(Value::as_str).unwrap_or("");
        Ok(Some(json!({
            "data": if is_text { String::from_utf8_lossy(&bytes).into_owned() } else { STANDARD.encode(&bytes) },
            "type": asset["type"], "extension": asset["extension"],
            "mimeType": if is_text { "text/css" } else { get_mime_type(extension) }, "isText": is_text
        })))
    }
    /// Read only the declared CSS file and perform Electron's URL substitution.
    /// The caller must never apply this text as GPUI styling.
    pub fn load_css(&self, theme: Option<&Theme>, requested: &Value) -> io::Result<Option<String>> {
        let Some(theme) = theme else { return Ok(None) };
        let declared = theme
            .get("cssFile")
            .and_then(Value::as_str)
            .and_then(normalize_css_path);
        let requested = requested.as_str().and_then(normalize_css_path);
        let Some(declared) = declared.filter(|p| Some(p) == requested.as_ref()) else {
            return Ok(None);
        };
        let directory = match self.theme_directory(theme) {
            Ok(Some(p)) => p,
            _ => return Ok(None),
        };
        let path = directory.join(declared);
        let path = match self.existing(&path) {
            Ok(p) if p.starts_with(&directory) => p,
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => return Ok(None),
            Err(e) => return Err(e),
            _ => return Ok(None),
        };
        let bytes = fs::read(path)?;
        let css = String::from_utf8_lossy(&bytes);
        static URL: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"url\(['"]?\./([^'")\s]+)['"]?\)"#).unwrap());
        static EXT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.[^.]+$").unwrap());
        Ok(Some(
            URL.replace_all(&css, |captures: &regex::Captures<'_>| {
                let name = EXT.replace(&captures[1], "");
                let name: String = name
                    .encode_utf16()
                    .map(|c| match c {
                        48..=57 | 65..=90 | 97..=122 => char::from_u32(u32::from(c)).unwrap(),
                        _ => '-',
                    })
                    .collect();
                format!("var(--asset-{name})")
            })
            .into_owned(),
        ))
    }
}
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "path outside authorized theme root or unsafe symlink",
    )
}
fn lexical_inside(base: &Path, path: &Path) -> io::Result<()> {
    if !path.is_absolute()
        || !path.starts_with(base)
        || path.components().any(|p| matches!(p, Component::ParentDir))
    {
        Err(denied())
    } else {
        Ok(())
    }
}
fn normalize_css_path(value: &str) -> Option<String> {
    let value = js_trim(value).replace('\\', "/");
    if value.is_empty()
        || value.contains('\0')
        || value.starts_with('/')
        || (value.as_bytes().get(1) == Some(&b':') && value.as_bytes().get(2) == Some(&b'/'))
    {
        return None;
    }
    let mut parts = Vec::new();
    for part in value.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    let path = parts.join("/");
    path.to_ascii_lowercase().ends_with(".css").then_some(path)
}
