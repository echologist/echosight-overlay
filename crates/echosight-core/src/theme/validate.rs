//! v1 validation of raw JSON, preserving TS message text and traversal order.
use super::helpers::{is_audio_asset_extension, js_entries, js_trim};
use super::v1::model::Theme;
use serde::Serialize;
use serde_json::Value;

const RECORD_FIELDS: &[&str] = &[
    "fonts",
    "effects",
    "transparency",
    "styles",
    "components",
    "backgrounds",
    "layout",
    "animations",
    "compatibility",
    "metadata",
];
const VARIANT_RECORD_FIELDS: &[&str] = &[
    "colors",
    "effects",
    "fonts",
    "styles",
    "components",
    "backgrounds",
    "layout",
    "animations",
];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Validation {
    /// Present exactly when errors is empty, even with warnings. No typed
    /// deserialization or normalization discards unknown/warning-only fields.
    pub theme: Option<Theme>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ValidationMessages {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn validate_theme(value: &Value) -> Validation {
    let Some(record) = value.as_object() else {
        return Validation {
            theme: None,
            errors: vec!["theme must be a JSON object".into()],
            warnings: vec![],
        };
    };
    let mut messages = ValidationMessages::default();
    for field in ["id", "name"] {
        if !record
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|s| !js_trim(s).is_empty())
        {
            messages
                .errors
                .push(format!("{field} must be a non-empty string"));
        }
    }
    optional_strings(
        record,
        &["author", "description", "version"],
        "",
        &mut messages.errors,
    );
    for field in ["isDefault", "supportsTransparency"] {
        if record.get(field).is_some_and(|v| !v.is_boolean()) {
            messages.errors.push(format!("{field} must be a boolean"));
        }
    }
    if let Some(value) = record.get("cssFile") {
        validate_file(Some(value), "cssFile", false, &mut messages.errors);
    }
    custom_css(record.get("customCSS"), "customCSS", &mut messages.errors);
    sounds(record.get("sounds"), "sounds", &mut messages);
    colors(record.get("colors"), "colors", &mut messages.errors);
    record_fields(record, RECORD_FIELDS, "", &mut messages.errors);
    if let Some(variants) = record.get("variants") {
        if let Some(variants) = variants.as_object() {
            for (id, config) in js_entries(variants) {
                if !config.is_object() {
                    messages.warnings.push(format!(
                        "variants.{id} must be an object; variant will be ignored"
                    ));
                }
            }
        } else {
            messages
                .warnings
                .push("variants must be an object; variants will be ignored".into());
        }
    }
    Validation {
        theme: messages.errors.is_empty().then(|| record.clone()),
        errors: messages.errors,
        warnings: messages.warnings,
    }
}

pub fn validate_theme_variant_config(
    theme_id: &str,
    variant_id: &str,
    value: &Value,
) -> ValidationMessages {
    let prefix = format!("variants.{variant_id}");
    let mut messages = ValidationMessages::default();
    let Some(record) = value.as_object() else {
        messages.errors.push(format!("{prefix} must be an object"));
        // The TS early return intentionally does not append a skipping warning.
        return messages;
    };
    optional_strings(
        record,
        &["name", "description"],
        &prefix,
        &mut messages.errors,
    );
    custom_css(
        record.get("customCSS"),
        &format!("{prefix}.customCSS"),
        &mut messages.errors,
    );
    sounds(
        record.get("sounds"),
        &format!("{prefix}.sounds"),
        &mut messages,
    );
    for field in VARIANT_RECORD_FIELDS {
        let path = format!("{prefix}.{field}");
        if *field == "colors" {
            colors(record.get(*field), &path, &mut messages.errors);
        } else if record.get(*field).is_some_and(|v| !v.is_object()) {
            messages.errors.push(format!("{path} must be an object"));
        }
    }
    if !messages.errors.is_empty() {
        messages
            .warnings
            .push(format!("Skipping invalid variant {theme_id}/{variant_id}"));
    }
    messages
}

fn field_path(prefix: &str, field: &str) -> String {
    if prefix.is_empty() {
        field.into()
    } else {
        format!("{prefix}.{field}")
    }
}

fn optional_strings(record: &Theme, fields: &[&str], prefix: &str, errors: &mut Vec<String>) {
    for field in fields {
        if record.get(*field).is_some_and(|v| !v.is_string()) {
            errors.push(format!("{} must be a string", field_path(prefix, field)));
        }
    }
}

fn record_fields(record: &Theme, fields: &[&str], prefix: &str, errors: &mut Vec<String>) {
    for field in fields {
        if record.get(*field).is_some_and(|v| !v.is_object()) {
            errors.push(format!("{} must be an object", field_path(prefix, field)));
        }
    }
}

fn custom_css(value: Option<&Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(value) = value else { return };
    if value.is_string() {
        return;
    }
    let Some(record) = value.as_object() else {
        errors.push(format!("{prefix} must be a string or object"));
        return;
    };
    for (key, styles) in js_entries(record) {
        let path = format!("{prefix}.{key}");
        if styles.is_string() {
            continue;
        }
        if let Some(record) = styles.as_object() {
            css_values(record, &path, errors);
        } else {
            errors.push(format!("{path} must be a CSS string or object"));
        }
    }
}

fn colors(value: Option<&Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(value) = value else { return };
    if let Some(record) = value.as_object() {
        css_values(record, prefix, errors);
    } else {
        errors.push(format!("{prefix} must be an object"));
    }
}

fn css_values(record: &Theme, prefix: &str, errors: &mut Vec<String>) {
    for (key, value) in js_entries(record) {
        let path = format!("{prefix}.{key}");
        if let Some(record) = value.as_object() {
            css_values(record, &path, errors);
        } else if !value.is_string() && !value.is_number() {
            errors.push(format!("{path} must be a CSS string or number"));
        }
    }
}

fn sounds(value: Option<&Value>, prefix: &str, messages: &mut ValidationMessages) {
    let Some(value) = value else { return };
    let Some(record) = value.as_object() else {
        messages.errors.push(format!("{prefix} must be an object"));
        return;
    };
    if record.get("enabled").is_some_and(|v| !v.is_boolean()) {
        messages
            .errors
            .push(format!("{prefix}.enabled must be a boolean"));
    }
    volume(
        record.get("volume"),
        &format!("{prefix}.volume"),
        &mut messages.errors,
    );
    let Some(events) = record.get("events") else {
        return;
    };
    let Some(events) = events.as_object() else {
        messages
            .errors
            .push(format!("{prefix}.events must be an object"));
        return;
    };
    for (name, definition) in js_entries(events) {
        let path = format!("{prefix}.events.{name}");
        if !matches!(
            name.as_str(),
            "taskCompleted" | "backgroundActivated" | "undo" | "redo"
        ) {
            messages.warnings.push(format!(
                "{path} is not a supported sound event; event will be ignored"
            ));
            continue;
        }
        if definition.is_string() {
            validate_file(Some(definition), &path, true, &mut messages.errors);
        } else if let Some(definition) = definition.as_object() {
            validate_file(
                definition.get("file"),
                &format!("{path}.file"),
                true,
                &mut messages.errors,
            );
            volume(
                definition.get("volume"),
                &format!("{path}.volume"),
                &mut messages.errors,
            );
        } else {
            messages
                .errors
                .push(format!("{path} must be a sound file string or object"));
        }
    }
}

fn volume(value: Option<&Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(value) = value else { return };
    if !value
        .as_f64()
        .is_some_and(|v| v.is_finite() && (0.0..=1.0).contains(&v))
    {
        errors.push(format!("{prefix} must be a number between 0 and 1"));
    }
}

fn validate_file(value: Option<&Value>, prefix: &str, audio: bool, errors: &mut Vec<String>) {
    let Some(value) = value
        .and_then(Value::as_str)
        .filter(|v| !js_trim(v).is_empty())
    else {
        errors.push(format!(
            "{prefix} must be a non-empty {}string",
            if audio { "sound file " } else { "" }
        ));
        return;
    };
    if unsafe_path(value) {
        errors.push(format!(
            "{prefix} must be a relative path inside the theme folder"
        ));
    } else if audio && value.contains(['/', '\\']) {
        errors.push(format!("{prefix} must be a file name in the theme folder"));
    } else if audio && !is_audio_asset_extension(extension(value)) {
        errors.push(format!("{prefix} must point to a supported audio file"));
    } else if !audio && !extension(value).eq_ignore_ascii_case(".css") {
        errors.push(format!("{prefix} must point to a .css file"));
    }
}

fn unsafe_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.contains('\0')
        || value.starts_with(['/', '\\'])
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
        || value.split(['/', '\\']).any(|segment| segment == "..")
}

// Node path.extname ignores trailing separators and leading dotfiles, unlike
// Path::extension in some edge cases. Use the host's Node path flavor; the
// security check above always recognizes both POSIX and Windows absolute paths.
fn extension(value: &str) -> &str {
    let separator = |c| c == '/' || (cfg!(windows) && c == '\\');
    let value = value.trim_end_matches(separator);
    let mut name = value.rsplit(separator).next().unwrap_or_default();
    if cfg!(windows)
        && name.as_bytes().get(1) == Some(&b':')
        && name.as_bytes()[0].is_ascii_alphabetic()
    {
        name = &name[2..];
    }
    if name == ".." {
        return "";
    }
    match name.rfind('.') {
        Some(index) if index > 0 => &name[index..],
        _ => "",
    }
}
