//! Read-only validation/discovery and explicitly lossy, no-overwrite conversion.
mod io;
use echosight_core::theme::{
    get_supported_theme_asset_extensions,
    v2::{self, ConversionInput, Diagnostic, Severity, Theme, ThemeFormat},
    validate_theme, validate_theme_variant_config,
};
use io::{Destination, Root, Source};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::Path,
};

pub const HELP: &str = "echosight-theme [--json] validate <file-or-theme-dir>\nechosight-theme [--json] convert <v1-file-or-theme-dir> <new-output-dir>\nechosight-theme [--json] list <explicit-themes-directory>\nOptions: --json (structured stdout), --help, -- (end options).\nExit: 0 error-free (warnings possible), 1 input/I/O/conversion error, 2 usage/unimplemented.\nConversion is structural, not visual equivalence. doctor is not implemented.\n";
type Result<T> = std::result::Result<T, Vec<Diagnostic>>;
fn diagnostic(
    code: &str,
    message: impl ToString,
    location: impl ToString,
    severity: Severity,
) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        message: message.to_string().chars().take(2048).collect(),
        location: location.to_string().chars().take(2048).collect(),
        severity,
        value: Value::Null,
    }
}
fn failure(code: &str, message: impl ToString) -> Vec<Diagnostic> {
    vec![diagnostic(code, message, "", Severity::Error)]
}
fn fs_error(e: std::io::Error) -> Vec<Diagnostic> {
    failure(
        if e.to_string().contains("budget") {
            "budget"
        } else {
            "io"
        },
        e,
    )
}
fn append(d: &mut Vec<Diagnostic>, added: impl IntoIterator<Item = Diagnostic>) {
    for item in added {
        if d.len() >= 512 {
            return;
        }
        if d.len() == 511 {
            d.push(diagnostic(
                "budget",
                "CLI diagnostic budget exceeded (512); results incomplete",
                "",
                Severity::Error,
            ));
            return;
        }
        d.push(item);
    }
}
fn has_errors(d: &[Diagnostic]) -> bool {
    d.iter().any(|d| d.severity == Severity::Error)
}

/// Return the exit status and one JSON value; main chooses human or JSON rendering.
pub fn run(args: impl IntoIterator<Item = OsString>) -> (i32, bool, Value) {
    let mut json_output = false;
    let mut positional = Vec::new();
    let mut options = true;
    let mut help = false;
    let mut bad = false;
    for arg in args {
        if options && arg == "--" {
            options = false;
        } else if options && arg == "--json" {
            json_output = true;
        } else if options && (arg == "--help" || arg == "-h") {
            help = true;
        } else if options && arg.to_string_lossy().starts_with('-') {
            bad = true;
        } else {
            positional.push(arg);
        }
    }
    if help && !bad {
        return (
            0,
            json_output,
            json!({"ok":true,"help":HELP,"diagnostics":[]}),
        );
    }
    let command = positional.first().and_then(|s| s.to_str()).unwrap_or("");
    let expected = match command {
        "validate" | "list" => 2,
        "convert" => 3,
        _ => 0,
    };
    if bad || expected == 0 || positional.len() != expected {
        let message = if command == "doctor" {
            "doctor is not implemented; no scan performed"
        } else {
            "invalid arguments; use --help"
        };
        return (
            2,
            json_output,
            json!({"ok":false,"command":command,"diagnostics":failure("usage",message)}),
        );
    }
    let input = Path::new(&positional[1]);
    let result = match command {
        "validate" => validate(input),
        "convert" => convert(input, Path::new(&positional[2])),
        "list" => list(input),
        _ => unreachable!(),
    };
    let mut output = match result {
        Ok(value) => value,
        Err(d) => json!({"ok":false,"diagnostics":d}),
    };
    output["command"] = json!(command);
    (
        if output["ok"] == true { 0 } else { 1 },
        json_output,
        output,
    )
}

/// Bound raw v1 input before legacy diagnostics, variant clones or conversion
/// filesystem work. Traverse opaque extension data too, without constructing paths.
fn preflight_v1(document: &Value) -> Result<()> {
    fn walk(v: &Value, depth: usize, left: &mut usize) -> bool {
        if depth > 64 || *left == 0 {
            return false;
        }
        *left -= 1;
        match v {
            Value::Object(o) => o
                .iter()
                .all(|(k, v)| k.len() <= 256 && walk(v, depth + 1, left)),
            Value::Array(a) => a.iter().all(|v| walk(v, depth + 1, left)),
            _ => true,
        }
    }
    if !walk(document, 0, &mut 4096) {
        return Err(failure(
            "budget",
            "v1 validation exceeds 4096 nodes, 64 levels or 256-byte keys",
        ));
    }
    Ok(())
}

fn validate_document(document: &Value) -> Result<(ThemeFormat, Vec<Diagnostic>)> {
    let format = v2::detect_format(document)?;
    let mut diagnostics = Vec::new();
    match format {
        ThemeFormat::V2 => {
            let parsed = Theme::parse(document)?;
            diagnostics.extend_from_slice(parsed.diagnostics());
        }
        ThemeFormat::V1 => {
            preflight_v1(document)?;
            let checked = validate_theme(document);
            append(
                &mut diagnostics,
                checked
                    .errors
                    .into_iter()
                    .map(|m| diagnostic("invalid-v1", m, "", Severity::Error)),
            );
            append(
                &mut diagnostics,
                checked
                    .warnings
                    .into_iter()
                    .map(|m| diagnostic("v1-warning", m, "", Severity::Warning)),
            );
            if let Some(variants) = document["variants"].as_object() {
                for (id, config) in variants {
                    let checked = validate_theme_variant_config(
                        document["id"].as_str().unwrap_or_default(),
                        id,
                        config,
                    );
                    append(
                        &mut diagnostics,
                        checked.errors.into_iter().map(|m| {
                            diagnostic(
                                "invalid-v1-variant",
                                m,
                                format!("/variants/{id}"),
                                Severity::Error,
                            )
                        }),
                    );
                    append(
                        &mut diagnostics,
                        checked.warnings.into_iter().map(|m| {
                            diagnostic(
                                "v1-warning",
                                m,
                                format!("/variants/{id}"),
                                Severity::Warning,
                            )
                        }),
                    );
                    if diagnostics.len() == 512 {
                        break;
                    }
                }
            }
        }
    }
    if has_errors(&diagnostics) {
        Err(diagnostics)
    } else {
        Ok((format, diagnostics))
    }
}
fn format_number(format: ThemeFormat) -> u8 {
    match format {
        ThemeFormat::V1 => 1,
        ThemeFormat::V2 => 2,
    }
}
fn validate(path: &Path) -> Result<Value> {
    let source = Source::open(path).map_err(fs_error)?;
    let (format, diagnostics) = validate_document(&source.document)?;
    Ok(
        json!({"ok":true,"format":format_number(format),"id":source.document["id"],"diagnostics":diagnostics}),
    )
}
fn supported(name: &str) -> bool {
    let extension = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s.to_ascii_lowercase()))
        .unwrap_or_default();
    get_supported_theme_asset_extensions().contains(&extension.as_str())
}
fn convert(input: &Path, output: &Path) -> Result<Value> {
    let source = Source::open(input).map_err(fs_error)?;
    if v2::detect_format(&source.document)? != ThemeFormat::V1 {
        return Err(failure(
            "conversion-input",
            "convert requires v1; v2 input is not downgraded",
        ));
    }
    preflight_v1(&source.document)?;
    let destination = Destination::new(output, &source.protected).map_err(fs_error)?;
    let css = source.document["cssFile"]
        .as_str()
        .map(|name| {
            let bytes = source.root.relative_read(name, io::TEXT_LIMIT)?;
            String::from_utf8(bytes).map_err(|e| io::error(format!("CSS must be UTF-8: {e}")))
        })
        .transpose()
        .map_err(fs_error)?;
    let mut assets = Vec::new();
    for entry in if source.folder {
        io::entries(&source.root).map_err(fs_error)?
    } else {
        Vec::new()
    } {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(failure("path", "non-UTF-8 theme filename is unsupported"));
        };
        if supported(name) {
            io::relative(name).map_err(fs_error)?;
            let canonical = source.root.existing(&entry.path()).map_err(fs_error)?;
            if !canonical.is_file() {
                return Err(failure(
                    "asset",
                    "supported asset name is not a regular file",
                ));
            }
            assets.push(name.to_owned());
        }
    }
    // The reviewed pure API takes one inherited asset set, not per-variant
    // directory overrides. Refuse such input instead of silently using base bytes.
    if source.folder
        && let Some(variants) = source.document["variants"].as_object()
    {
        if variants.len() > io::ENTRY_LIMIT {
            return Err(failure("budget", "variant directory work exceeds 1024"));
        }
        for id in variants.keys() {
            let relative = format!("variants/{id}");
            io::relative(&relative).map_err(fs_error)?;
            let path = source.root.path().join(&relative);
            match std::fs::symlink_metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(fs_error(e)),
                Ok(_) => {
                    source.root.existing(&path).map_err(fs_error)?;
                }
            }
            let variant_root = Root::new(&path).map_err(fs_error)?;
            for entry in io::entries(&variant_root).map_err(fs_error)? {
                if supported(&entry.file_name().to_string_lossy()) {
                    source.root.existing(&entry.path()).map_err(fs_error)?;
                    return Err(vec![diagnostic(
                        "variant-assets",
                        "per-variant asset directory overrides require manual v2 authoring; base asset inheritance would change this variant",
                        format!("{relative}/{}", entry.file_name().to_string_lossy()),
                        Severity::Error,
                    )]);
                }
            }
        }
    }
    let conversion = v2::convert_v1(ConversionInput {
        document: &source.document,
        css_file: css.as_deref(),
        asset_files: &assets,
    })?;
    if has_errors(&conversion.diagnostics) {
        return Err(conversion.diagnostics);
    }
    Theme::parse(&conversion.theme)?;
    // Only known output image and sound positions, not opaque metadata/extension keys.
    let mut needed = BTreeSet::new();
    fn image(value: &Value, needed: &mut BTreeSet<String>) {
        if let Some(asset) = value["asset"].as_str() {
            needed.insert(asset.into());
        }
        if let Some(states) = value["states"].as_object() {
            for v in states.values() {
                image(v, needed);
            }
        }
    }
    fn style(value: &Value, needed: &mut BTreeSet<String>) {
        image(&value["image"], needed);
        if let Some(states) = value["states"].as_object() {
            for v in states.values() {
                style(v, needed);
            }
        }
    }
    fn collect(doc: &Value, needed: &mut BTreeSet<String>) {
        if let Some(images) = doc["images"].as_object() {
            for v in images.values() {
                image(v, needed);
            }
        }
        if let Some(slots) = doc["slots"].as_object() {
            for v in slots.values() {
                style(v, needed);
            }
        }
        if let Some(events) = doc["sounds"]["events"].as_object() {
            for v in events.values() {
                if let Some(file) = v.as_str().or_else(|| v["file"].as_str()) {
                    needed.insert(file.into());
                }
            }
        }
        if let Some(variants) = doc["variants"].as_object() {
            for v in variants.values() {
                collect(v, needed);
            }
        }
    }
    collect(&conversion.theme, &mut needed);
    let mut payload = BTreeMap::new();
    let mut total = 0;
    for name in needed {
        if !supported(&name) {
            return Err(failure(
                "asset",
                format!("unsupported asset extension: {name:?}"),
            ));
        }
        let bytes = source
            .root
            .relative_read(&name, io::ASSET_LIMIT)
            .map_err(fs_error)?;
        total += bytes.len();
        if total > io::TOTAL_LIMIT {
            return Err(failure(
                "budget",
                "total asset byte budget exceeded (64 MiB)",
            ));
        }
        payload.insert(name, bytes);
    }
    let theme_bytes =
        serde_json::to_vec_pretty(&conversion.theme).map_err(|e| failure("json", e))?;
    let saved: Value = serde_json::from_slice(&theme_bytes).map_err(|e| failure("json", e))?;
    Theme::parse(&saved)?;
    payload.insert("theme.json".into(), theme_bytes);
    payload.insert(
        "CONVERSION-REPORT.md".into(),
        conversion.report_markdown().into_bytes(),
    );
    source.root.check().map_err(fs_error)?;
    destination.publish(&payload).map_err(fs_error)?;
    Ok(
        json!({"ok":true,"format":2,"id":conversion.theme["id"],"output":output,"provisional":conversion.provisional,"diagnostics":conversion.diagnostics}),
    )
}
fn list(path: &Path) -> Result<Value> {
    let root = Root::new(path).map_err(fs_error)?;
    let entries = io::entries(&root).map_err(fs_error)?;
    let mut themes: Vec<Value> = Vec::new();
    let mut diagnostics = Vec::new();
    let mut bytes = 0;
    for entry in entries {
        let kind = entry.file_type().map_err(fs_error)?;
        if !(kind.is_dir()
            || kind.is_file() && entry.file_name().to_string_lossy().ends_with(".json"))
        {
            continue;
        }
        let file = if kind.is_dir() {
            entry.path().join("theme.json")
        } else {
            entry.path()
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let loaded = (|| {
            let canonical = root.existing(&file).map_err(fs_error)?;
            // A child theme folder is its own authorized asset/document root.
            let source_root = Root::new(file.parent().unwrap()).map_err(fs_error)?;
            source_root.existing(&file).map_err(fs_error)?;
            let data = root.read(&canonical, io::TEXT_LIMIT).map_err(fs_error)?;
            bytes += data.len();
            if bytes > 16 * io::TEXT_LIMIT {
                return Err(failure(
                    "budget",
                    "list input byte budget exceeded (16 MiB)",
                ));
            }
            let doc: Value = serde_json::from_slice(&data).map_err(|e| failure("json", e))?;
            let (format, d) = validate_document(&doc)?;
            append(
                &mut diagnostics,
                d.into_iter().map(|mut d| {
                    d.location = format!("{name}{}", d.location);
                    d
                }),
            );
            let mut add = |id: Value, name: Value, variant: Option<&str>| {
                let item = json!({"id":id,"name":name,"format":format_number(format),"source":entry.file_name().to_string_lossy(),"variant":variant});
                if let Some(existing) = themes.iter_mut().find(|t| t["id"] == item["id"]) {
                    *existing = item;
                } else {
                    themes.push(item);
                }
            };
            if let Some(variants) = doc["variants"].as_object() {
                let mut keys: Vec<_> = variants.keys().collect();
                // Match Object.entries canonical numeric keys before insertion-order strings.
                keys.sort_by_key(|k| {
                    k.parse::<u32>()
                        .ok()
                        .filter(|n| *n != u32::MAX && n.to_string() == **k)
                        .map_or((1, 0), |n| (0, n))
                });
                for id in keys {
                    add(
                        json!(format!("{}-{id}", doc["id"].as_str().unwrap_or_default())),
                        variants[id]
                            .get("name")
                            .filter(|v| v.as_str().is_some_and(|s| !s.is_empty()))
                            .cloned()
                            .unwrap_or_else(|| {
                                json!(format!(
                                    "{} ({id})",
                                    doc["name"].as_str().unwrap_or_default()
                                ))
                            }),
                        Some(id),
                    );
                }
            }
            add(doc["id"].clone(), doc["name"].clone(), None);
            Ok::<_, Vec<Diagnostic>>(())
        })();
        if let Err(d) = loaded {
            append(
                &mut diagnostics,
                d.into_iter().map(|mut d| {
                    d.location = format!("{name}{}", d.location);
                    d
                }),
            );
        }
        if diagnostics.len() == 512 || bytes > 16 * io::TEXT_LIMIT {
            break;
        }
    }
    Ok(json!({"ok":!has_errors(&diagnostics),"themes":themes,"diagnostics":diagnostics}))
}
