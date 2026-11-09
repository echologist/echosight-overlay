//! v1 filesystem discovery. Open maps preserve unknown fields and TS Map order.
use super::{
    assets::ThemeRoot,
    helpers::{
        create_variant_theme, get_asset_type, get_supported_theme_asset_extensions, js_entries,
    },
    sync::{read_json, sorted_entries, sync_bundled},
    v1::model::Theme,
    validate::{validate_theme, validate_theme_variant_config},
};
use serde_json::{Value, json};
use std::{fs, io, path::Path};

#[derive(Debug, Clone)]
pub struct ThemeFailure {
    pub context: String,
    pub message: String,
}
#[derive(Debug)]
pub struct ThemeLibrary {
    root: ThemeRoot,
    /// In discovery order. Replacing an existing id retains its original slot.
    pub themes: Vec<Theme>,
    pub warnings: Vec<String>,
    pub failures: Vec<ThemeFailure>,
}
impl ThemeLibrary {
    /// The caller creates/selects this directory explicitly; never guesses a
    /// user-data path. Use initialize for bundled sync or reload for custom-only.
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            root: ThemeRoot::new(path)?,
            themes: Vec::new(),
            warnings: Vec::new(),
            failures: Vec::new(),
        })
    }
    pub fn root(&self) -> &ThemeRoot {
        &self.root
    }
    pub fn get(&self, id: &str) -> Option<&Theme> {
        self.themes
            .iter()
            .find(|t| t.get("id").and_then(Value::as_str) == Some(id))
    }
    /// loaded_at is an injected ISO timestamp, making the filesystem result
    /// deterministic without depending on a UI or global clock.
    pub fn initialize(&mut self, bundled: &ThemeRoot, loaded_at: &str) {
        self.warnings.clear();
        self.failures.clear();
        self.themes.clear();
        if let Err(error) = sync_bundled(bundled, &self.root) {
            self.failure("Failed to copy default themes:", error);
        }
        self.load(loaded_at);
    }
    pub fn reload(&mut self, loaded_at: &str) {
        self.warnings.clear();
        self.failures.clear();
        self.themes.clear();
        self.load(loaded_at);
    }
    fn failure(&mut self, context: impl Into<String>, error: impl std::fmt::Display) {
        self.failures.push(ThemeFailure {
            context: context.into(),
            message: error.to_string(),
        });
    }
    fn insert(&mut self, theme: Theme) {
        if let Some(existing) = self
            .themes
            .iter_mut()
            .find(|t| t.get("id") == theme.get("id"))
        {
            *existing = theme;
        } else {
            self.themes.push(theme);
        }
    }
    fn load(&mut self, loaded_at: &str) {
        let entries = match sorted_entries(self.root.path()) {
            Ok(entries) => entries,
            Err(e) => {
                self.failure("Failed to load themes:", e);
                return;
            }
        };
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let kind = match entry.file_type() {
                Ok(k) => k,
                Err(e) => {
                    self.failure("Failed to load themes:", e);
                    return;
                }
            };
            let folder = kind.is_dir();
            if !(folder || kind.is_file() && name.ends_with(".json")) {
                continue;
            }
            let path = if folder {
                entry.path().join("theme.json")
            } else {
                entry.path()
            };
            let source = format!("theme {} {name}", if folder { "folder" } else { "file" });
            if folder && self.root.existing(&path).is_err() {
                self.warnings
                    .push(format!("No theme.json found in folder: {name}"));
                continue;
            }
            let value = match read_json(&self.root, &path) {
                Ok(value) => value,
                Err(e) => {
                    self.failure(format!("Failed to load {source}:"), e);
                    continue;
                }
            };
            let validation = validate_theme(&value);
            self.validation_warnings(&source, &validation.warnings);
            let Some(mut theme) = validation.theme else {
                self.warnings.push(format!(
                    "Invalid {source}: {}",
                    validation.errors.join("; ")
                ));
                continue;
            };
            theme.insert("assets".into(), json!({}));
            theme.insert("path".into(), json!(path));
            if folder {
                let folder_path = entry.path();
                theme.insert(
                    "assets".into(),
                    Value::Object(self.load_assets(&folder_path)),
                );
                theme.insert("folderPath".into(), json!(folder_path));
                self.metadata(&mut theme, &folder_path, loaded_at);
                self.variants(&theme, &folder_path);
            }
            self.insert(theme);
        }
    }
    fn validation_warnings(&mut self, source: &str, warnings: &[String]) {
        for warning in warnings {
            self.warnings
                .push(format!("Theme warning in {source}: {warning}"));
        }
    }
    fn load_assets(&mut self, directory: &Path) -> Theme {
        let mut assets = Theme::new();
        let entries = match self
            .root
            .existing(directory)
            .and_then(|p| sorted_entries(&p))
        {
            Ok(e) => e,
            Err(e) => {
                self.failure("Failed to load theme assets:", e);
                return assets;
            }
        };
        for entry in entries {
            let file = entry.file_name().to_string_lossy().into_owned();
            let extension = entry
                .path()
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy().to_ascii_lowercase()))
                .unwrap_or_default();
            if !get_supported_theme_asset_extensions().contains(&extension.as_str()) {
                continue;
            }
            let path = directory.join(&file);
            // Canonicalize every asset; don't advertise outside links or directories.
            if !self
                .root
                .existing_in(directory, &path)
                .is_ok_and(|p| p.is_file())
            {
                continue;
            }
            // Node basename(file, lowercasedExtension) removes only an exact suffix.
            let name = file.strip_suffix(&extension).unwrap_or(&file);
            assets.insert(name.to_owned(), json!({ "path":path, "relativePath":format!("themes/{}/{file}", directory.file_name().unwrap_or_default().to_string_lossy()), "type": get_asset_type(name, &extension), "extension":extension }));
        }
        assets
    }
    fn metadata(&mut self, theme: &mut Theme, directory: &Path, loaded_at: &str) {
        for file in [
            "preview.png",
            "preview.jpg",
            "preview.jpeg",
            "screenshot.png",
        ] {
            let path = directory.join(file);
            if self
                .root
                .existing_in(directory, &path)
                .is_ok_and(|p| p.is_file())
            {
                theme.insert("preview".into(), json!({ "path":path,"relativePath":format!("themes/{}/{file}",directory.file_name().unwrap_or_default().to_string_lossy()) }));
                break;
            }
        }
        for file in ["README.md", "readme.txt", "info.txt"] {
            if let Ok(path) = self.root.existing_in(directory, &directory.join(file))
                && let Ok(bytes) = fs::read(path)
            {
                theme.insert(
                    "documentation".into(),
                    json!({"file":file,"content":String::from_utf8_lossy(&bytes)}),
                );
                break;
            }
        }
        theme.insert("metadata".into(), json!({
            "loadedAt":loaded_at, "assetCount":theme["assets"].as_object().map_or(0, |v|v.len()),
            "hasVariants":theme.get("variants").is_some_and(|v| v.as_object().is_some_and(|m| !m.is_empty()) || v.as_array().is_some_and(|a| !a.is_empty())),
            "hasPreview":theme.get("preview").is_some_and(truthy), "hasDocumentation":theme.get("documentation").is_some_and(truthy),
            "isAdvanced":(["animations","layout","backgrounds"].iter().any(|k|theme.get(*k).is_some_and(truthy))),
            "compatibility":theme.get("compatibility").filter(|v|truthy(v)).cloned().unwrap_or(json!({"minVersion":"1.0.0","maxVersion":"*"}))
        }));
    }
    fn variants(&mut self, theme: &Theme, directory: &Path) {
        // Library's isRecord includes arrays, unlike validator/helper isRecord.
        let variants = match theme.get("variants") {
            Some(Value::Object(v)) => v.clone(),
            Some(Value::Array(v)) => v
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v.clone()))
                .collect(),
            _ => return,
        };
        let id = theme["id"].as_str().unwrap_or_default();
        let mut staged = Vec::new();
        for (variant_id, config) in js_entries(&variants) {
            if !config.is_object() && !config.is_array() {
                self.warnings
                    .push(format!("Invalid variant config for {id}/{variant_id}"));
                continue;
            }
            let validation = validate_theme_variant_config(id, variant_id, config);
            self.validation_warnings(&format!("variant {id}/{variant_id}"), &validation.warnings);
            if !validation.errors.is_empty() {
                self.warnings.push(format!(
                    "Invalid variant {id}/{variant_id}: {}",
                    validation.errors.join("; ")
                ));
                continue;
            }
            let mut variant = create_variant_theme(theme, variant_id, config.as_object().unwrap());
            // Invalid variant paths abort the entire staged batch in TS.
            let Some(path) = variant_path(directory, variant_id) else {
                self.failure("Failed to load theme variants:", "unsafe variant path");
                return;
            };
            if self.root.existing_in(directory, &path).is_ok() {
                let assets = self.load_assets(&path);
                variant
                    .get_mut("assets")
                    .and_then(Value::as_object_mut)
                    .unwrap()
                    .extend(assets);
            }
            let mut metadata = theme["metadata"].as_object().cloned().unwrap_or_default();
            metadata.insert("isVariant".into(), json!(true));
            metadata.insert("baseTheme".into(), json!(id));
            metadata.insert("variantId".into(), json!(variant_id));
            variant.insert("metadata".into(), Value::Object(metadata));
            staged.push(variant);
        }
        for theme in staged {
            self.insert(theme);
        }
    }
}
fn variant_path(directory: &Path, id: &str) -> Option<std::path::PathBuf> {
    use std::path::Component;
    if id.contains('\0') {
        return None;
    }
    // Backslashes are path separators on Windows. Treat them as such on every
    // host to avoid importing a traversal that becomes unsafe after migration.
    let joined = directory.join("variants").join(id.replace('\\', "/"));
    let mut result = std::path::PathBuf::new();
    for component in joined.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            _ => result.push(component),
        }
    }
    let relative = result.strip_prefix(directory).ok()?;
    if relative.to_string_lossy().starts_with("..") {
        return None;
    }
    Some(result)
}
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Number(v) => v.as_f64() != Some(0.0),
        Value::String(v) => !v.is_empty(),
        _ => true,
    }
}
