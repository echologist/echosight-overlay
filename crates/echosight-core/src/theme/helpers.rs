//! Pure JSON helpers from `themeHelpers.ts` and `themeLibrary.ts`.
use super::v1::model::Theme;
use serde_json::Value;

const EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".mp3", ".ogg", ".wav", ".m4a",
];

pub fn get_supported_theme_asset_extensions() -> &'static [&'static str] {
    EXTENSIONS
}

pub fn is_audio_asset_extension(extension: &str) -> bool {
    matches!(
        extension.to_ascii_lowercase().as_str(),
        ".mp3" | ".ogg" | ".wav" | ".m4a"
    )
}

pub fn get_asset_type(asset_name: &str, extension: &str) -> &'static str {
    if is_audio_asset_extension(extension) {
        return "sound";
    }
    let name = asset_name.to_lowercase();
    if name.contains("background") || name.contains("bg") {
        return "background";
    }
    for kind in ["button", "progress", "icon", "texture", "border"] {
        if name.contains(kind) {
            return kind;
        }
    }
    "misc"
}

/// MIME lookup for a Node `path.extname` result (empty or beginning with `.`).
/// TS parity is bounded to these extension strings, case-insensitively. The
/// production TS library constructs asset extensions with `path.extname`; JSON
/// declarations cannot override them. Bare prototype names such as `__proto__`
/// and `constructor` are outside this contract: TS can return an object/function,
/// while this total string API returns `application/octet-stream`. That fallback
/// is not an assertion of arbitrary-string TS parity or an accepted product delta.
pub fn get_mime_type(extension: &str) -> &'static str {
    match extension.to_ascii_lowercase().as_str() {
        ".png" => "image/png",
        ".jpg" | ".jpeg" => "image/jpeg",
        ".gif" => "image/gif",
        ".svg" => "image/svg+xml",
        ".webp" => "image/webp",
        ".mp3" => "audio/mpeg",
        ".ogg" => "audio/ogg",
        ".wav" => "audio/wav",
        ".m4a" => "audio/mp4",
        _ => "application/octet-stream",
    }
}

/// JSON-object subset of `defu(overrides, defaults)` (not an ordinary merge).
/// Arrays concatenate overrides first; null overrides are skipped; defaults
/// retain nulls. `__proto__` and `constructor` are skipped only on the override
/// side, matching the installed dependency, including nested recursion.
pub fn deep_merge_defu(overrides: &Theme, defaults: &Theme) -> Theme {
    let mut result = defaults.clone();
    for (key, value) in js_entries(overrides) {
        if key == "__proto__" || key == "constructor" || value.is_null() {
            continue;
        }
        let merged = match (value, result.get(key)) {
            (Value::Array(first), Some(Value::Array(last))) => {
                Value::Array(first.iter().chain(last).cloned().collect())
            }
            (Value::Object(first), Some(Value::Object(last))) => {
                Value::Object(deep_merge_defu(first, last))
            }
            _ => value.clone(),
        };
        result.insert(key.clone(), merged);
    }
    result
}

/// Create a variant without modifying the base. Inputs are open JSON objects;
/// callers should validate first. Unknown base keys survive, but unknown
/// override keys are deliberately ignored, just as in `createVariantTheme`.
pub fn create_variant_theme(base: &Theme, variant_id: &str, config: &Theme) -> Theme {
    let mut result = base.clone();
    // Validated base themes always have string id and name.
    let id = base.get("id").and_then(Value::as_str).unwrap_or_default();
    let name = base.get("name").and_then(Value::as_str).unwrap_or_default();
    result.insert("id".into(), Value::String(format!("{id}-{variant_id}")));
    result.insert(
        "name".into(),
        Value::String(
            nonempty_string(config.get("name"))
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{name} ({variant_id})")),
        ),
    );
    let description = nonempty_string(base.get("description")).unwrap_or(name);
    result.insert(
        "description".into(),
        Value::String(
            nonempty_string(config.get("description"))
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{description} - {variant_id} variant")),
        ),
    );
    for field in [
        "colors",
        "effects",
        "fonts",
        "styles",
        "components",
        "backgrounds",
        "layout",
        "animations",
        "customCSS",
        "sounds",
    ] {
        let override_value = config.get(field);
        // The last five assignments in TS are truthiness-gated; the first five
        // always run. Missing JS undefined is represented by an absent key.
        if matches!(
            field,
            "backgrounds" | "layout" | "animations" | "customCSS" | "sounds"
        ) && !override_value.is_some_and(js_truthy)
        {
            continue;
        }
        if let Some(value) = merge_section(base.get(field), override_value) {
            result.insert(field.into(), value);
        }
    }
    result.shift_remove("variants");
    result
}

fn nonempty_string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn merge_section(base: Option<&Value>, overrides: Option<&Value>) -> Option<Value> {
    let Some(overrides) = overrides.filter(|v| !v.is_null()) else {
        return base.cloned();
    };
    Some(match (base, overrides) {
        (Some(Value::Object(base)), Value::Object(overrides)) => {
            Value::Object(deep_merge_defu(overrides, base))
        }
        _ => overrides.clone(),
    })
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|v| v != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// Numeric difference of the first unequal `parseInt(part, 10)` component,
/// not semver ordering. Missing trailing components are zero. Invalid versions
/// compare equal; suffix text after a numeric prefix is ignored. f64 intentionally
/// mirrors JS Number rounding (including integers beyond the safe range). Parsed
/// components must be finite, but their difference can overflow to +/-infinity.
pub fn compare_theme_version(left: &Value, right: &Value) -> f64 {
    let (Some(left), Some(right)) = (parse_version(left), parse_version(right)) else {
        return 0.0;
    };
    for i in 0..left.len().max(right.len()) {
        let left = left.get(i).copied().unwrap_or(0.0);
        let right = right.get(i).copied().unwrap_or(0.0);
        if left != right {
            return left - right;
        }
    }
    0.0
}

fn parse_version(value: &Value) -> Option<Vec<f64>> {
    value
        .as_str()?
        .split('.')
        .map(|part| {
            let part = js_trim(part);
            let digits = part.strip_prefix(['+', '-']).unwrap_or(part);
            let count = digits.bytes().take_while(u8::is_ascii_digit).count();
            if count == 0 {
                return None;
            }
            let signed_len = part.len() - digits.len() + count;
            let value = part[..signed_len].parse::<f64>().ok()?;
            value.is_finite().then_some(value)
        })
        .collect()
}

/// JS whitespace differs from Rust's Unicode White_Space (BOM vs NEL).
pub(super) fn js_trim(value: &str) -> &str {
    value.trim_matches(|c| {
        matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}'
        | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}'
        | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
    })
}

/// Object.entries enumerates canonical uint32 array-index keys first, even
/// when they appeared later in JSON; other keys retain insertion order.
pub(super) fn js_entries(map: &Theme) -> Vec<(&String, &Value)> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by_key(|(key, _)| {
        key.parse::<u32>()
            .ok()
            .filter(|n| *n != u32::MAX && n.to_string() == **key)
            .map_or((1, 0), |n| (0, n))
    });
    entries
}
