//! Byte-preserving bundled sync. Never parse/rewrite a user's custom theme.
use super::{assets::ThemeRoot, compare_theme_version};
use crate::storage::parse_json;
use serde_json::Value;
use std::{fs, io, path::Path};

pub(super) fn read_json(root: &ThemeRoot, path: &Path) -> io::Result<Value> {
    let bytes = fs::read(root.existing(path)?)?;
    parse_json(&String::from_utf8_lossy(&bytes)).map_err(io::Error::other)
}

/// Only a matching identity and strictly newer numeric dotted version permit
/// replacement. Absence of author equals absence; null and absence differ.
pub fn should_update(source: &Value, target: &Value) -> bool {
    let Some(source) = source.as_object() else {
        return false;
    };
    let Some(target) = target.as_object() else {
        return false;
    };
    let Some(id) = source
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    else {
        return false;
    };
    if target.get("id").and_then(Value::as_str) != Some(id)
        || source.get("author") != target.get("author")
    {
        return false;
    }
    // JS strict equality never equates independently parsed object/array authors.
    if source
        .get("author")
        .is_some_and(|v| v.is_object() || v.is_array())
    {
        return false;
    }
    compare_theme_version(
        source.get("version").unwrap_or(&Value::Null),
        target.get("version").unwrap_or(&Value::Null),
    ) > 0.0
}

/// Both roots must have been explicitly authorized. Existing destination
/// symlinks are rejected (including dangling links). Unknown/custom files are
/// retained during folder updates, just like the source recursive copy.
pub fn sync_bundled(source: &ThemeRoot, target: &ThemeRoot) -> io::Result<()> {
    for entry in sorted_entries(source.path())? {
        let kind = entry.file_type()?;
        let name = entry.file_name();
        let source_path = entry.path();
        let target_path = target.path().join(&name);
        if !(kind.is_dir() || kind.is_file() && name.to_string_lossy().ends_with(".json")) {
            continue;
        }
        target.destination(&target_path)?;
        let exists = fs::symlink_metadata(&target_path).is_ok();
        let update = if exists {
            let (s, t) = if kind.is_dir() {
                (
                    source_path.join("theme.json"),
                    target_path.join("theme.json"),
                )
            } else {
                (source_path.clone(), target_path.clone())
            };
            match (read_json(source, &s), read_json(target, &t)) {
                (Ok(s), Ok(t)) => should_update(&s, &t),
                _ => false,
            }
        } else {
            true
        };
        if update {
            copy_tree(source, target, &source_path, &target_path)?;
        }
    }
    Ok(())
}
fn copy_tree(source: &ThemeRoot, target: &ThemeRoot, from: &Path, to: &Path) -> io::Result<()> {
    target.destination(to)?;
    let meta = fs::symlink_metadata(from)?;
    // Do not follow source symlinks, even inside the root: cycles and dangling
    // links cannot be safely reproduced by Electron's copyFile fallback.
    if meta.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "symlink in bundled theme",
        ));
    }
    let from = source.existing(from)?;
    if meta.is_dir() {
        fs::create_dir_all(to)?;
        for entry in sorted_entries(&from)? {
            copy_tree(source, target, &entry.path(), &to.join(entry.file_name()))?;
        }
    } else if meta.is_file() {
        fs::copy(from, to)?;
    }
    Ok(())
}
pub(super) fn sorted_entries(path: &Path) -> io::Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}
