//! The shipped authoring schema is also the structural validator's source of truth.
use super::{Diagnostic, Severity, budget::Budget, color::Rgba};
use serde_json::Value;
use std::sync::LazyLock;

pub const SCHEMA: &str = include_str!("../../../schema/theme-v2.schema.json");
pub static CONTRACT: LazyLock<Value> =
    LazyLock::new(|| serde_json::from_str(SCHEMA).expect("embedded theme schema"));

pub(super) fn issue(
    out: &mut Vec<Diagnostic>,
    location: &str,
    code: &str,
    message: &str,
    value: &Value,
    severity: Severity,
) {
    if diagnostic_limit(out) {
        return;
    }
    let include = Budget::new(64, 4096).charge(value, 1);
    out.push(Diagnostic {
        severity,
        location: bounded_text(location),
        code: bounded_text(code),
        message: if include {
            bounded_text(message)
        } else {
            format!("{}; source value omitted for safety", bounded_text(message))
        },
        value: if include { value.clone() } else { Value::Null },
    });
}
/// Never silently drop errors behind a flood of warnings.
fn diagnostic_limit(out: &mut [Diagnostic]) -> bool {
    if out.len() < 512 {
        return false;
    }
    if let Some(last) = out.last_mut() {
        *last = Diagnostic {
            severity: Severity::Error,
            location: String::new(),
            code: "budget".into(),
            message: "diagnostic count exceeds 512; remaining diagnostics omitted".into(),
            value: Value::Null,
        };
    }
    true
}
pub(super) fn append(out: &mut Vec<Diagnostic>, incoming: impl IntoIterator<Item = Diagnostic>) {
    for diagnostic in incoming {
        if diagnostic_limit(out) {
            break;
        }
        out.push(diagnostic);
    }
}
pub(super) fn bounded_text(text: &str) -> String {
    if text.len() <= 1024 {
        return text.to_owned();
    }
    let mut end = 1021;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &text[..end])
}
pub(super) fn path(parent: &str, key: &str) -> String {
    bounded_text(&format!(
        "{}/{}",
        bounded_text(parent),
        bounded_text(key).replace('~', "~0").replace('/', "~1")
    ))
}
pub(super) fn safe_asset(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(['\\', ':', '\0'])
        && !s.starts_with('/')
        && s.split('/').all(|p| !matches!(p, "" | "." | ".."))
}
/// Select the first structurally valid union branch in schema order. The validator,
/// reference walker, semantic checks and renderer must interpret the same branch.
pub(super) fn selected_schema<'a>(v: &Value, schema: &'a Value) -> Option<&'a Value> {
    fn select<'a>(v: &Value, schema: &'a Value, depth: usize) -> Option<&'a Value> {
        if depth > 96 {
            return None;
        }
        if let Some(reference) = schema["$ref"].as_str() {
            return select(
                v,
                CONTRACT.pointer(reference.strip_prefix('#')?)?,
                depth + 1,
            );
        }
        if let Some(options) = schema["anyOf"].as_array() {
            for option in options {
                let mut diagnostics = vec![];
                check(v, option, "", &mut diagnostics, depth + 1);
                if !diagnostics.iter().any(|d| d.severity == Severity::Error) {
                    return select(v, option, depth + 1);
                }
            }
            return None;
        }
        schema.is_object().then_some(schema)
    }
    select(v, schema, 0)
}

/// Child positions of a selected schema. Boolean/open extension fields have no
/// interpreted children; schema-valued additionalProperties (token maps) do.
pub(super) fn child_schema<'a>(schema: &'a Value, key: &str) -> Option<&'a Value> {
    let child = if schema["type"] == "array" {
        key.parse::<usize>()
            .ok()
            .and_then(|i| schema["prefixItems"].get(i))
            .or_else(|| schema.get("items"))
    } else {
        schema["properties"]
            .get(key)
            .or_else(|| schema.get("additionalProperties"))
    };
    child.filter(|s| s.is_object())
}

pub(super) fn reference_position(schema: &Value) -> bool {
    // selected_schema has already followed the $ref and chosen the union branch.
    std::ptr::eq(schema, &CONTRACT["$defs"]["reference"])
}

/// JSON Schema compares numbers by value, not serde_json's integer/float tag.
/// Preserve exact integer comparisons rather than rounding large integers to f64.
pub(super) fn json_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            if a == b {
                return true;
            }
            let integer_float_equal = |integer: &serde_json::Number, float: f64| {
                if float.fract() != 0. {
                    return false;
                }
                if let Some(n) = integer.as_i64() {
                    float >= i64::MIN as f64 && float < -(i64::MIN as f64) && float as i64 == n
                } else if let Some(n) = integer.as_u64() {
                    float >= 0. && float < -(i64::MIN as f64) * 2. && float as u64 == n
                } else {
                    false
                }
            };
            if a.is_f64() && !b.is_f64() {
                a.as_f64().is_some_and(|f| integer_float_equal(b, f))
            } else if b.is_f64() && !a.is_f64() {
                b.as_f64().is_some_and(|f| integer_float_equal(a, f))
            } else {
                false
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| json_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, a)| b.get(k).is_some_and(|b| json_equal(a, b)))
        }
        _ => a == b,
    }
}

pub(super) fn check(v: &Value, schema: &Value, p: &str, out: &mut Vec<Diagnostic>, depth: usize) {
    if diagnostic_limit(out) {
        return;
    }
    if depth > 96 {
        issue(
            out,
            p,
            "depth",
            "maximum schema/value depth exceeded",
            v,
            Severity::Error,
        );
        return;
    }
    if schema == &Value::Bool(false) {
        issue(
            out,
            p,
            "forbidden",
            "property is forbidden; convert v1 CSS before loading format 2",
            v,
            Severity::Error,
        );
        return;
    }
    if let Some(reference) = schema["$ref"].as_str() {
        if let Some(s) = CONTRACT.pointer(reference.trim_start_matches('#')) {
            check(v, s, p, out, depth + 1);
        }
        return;
    }
    if let Some(options) = schema["anyOf"].as_array() {
        let mut best = None;
        for s in options {
            let mut d = vec![];
            check(v, s, p, &mut d, depth + 1);
            if !d.iter().any(|d| d.severity == Severity::Error) {
                append(out, d);
                return;
            }
            if best
                .as_ref()
                .is_none_or(|b: &Vec<Diagnostic>| d.len() < b.len())
            {
                best = Some(d);
            }
        }
        issue(
            out,
            p,
            "shape",
            "value does not match any supported shape",
            v,
            Severity::Error,
        );
        return;
    }
    let invalid_type = match schema["type"].as_str() {
        Some("object") => !v.is_object(),
        Some("array") => !v.is_array(),
        Some("string") => !v.is_string(),
        Some("number") => !v.is_number(),
        Some("boolean") => !v.is_boolean(),
        Some("null") => !v.is_null(),
        _ => false,
    };
    if invalid_type {
        issue(
            out,
            p,
            "type",
            &format!("expected {}", schema["type"]),
            v,
            Severity::Error,
        );
        return;
    }
    if schema.get("const").is_some_and(|c| !json_equal(c, v))
        || schema["enum"]
            .as_array()
            .is_some_and(|a| !a.iter().any(|c| json_equal(c, v)))
    {
        issue(
            out,
            p,
            "enum",
            "value is not in the supported vocabulary",
            v,
            Severity::Error,
        );
    }
    if let Some(n) = v.as_f64()
        && (!n.is_finite()
            || schema["minimum"].as_f64().is_some_and(|m| n < m)
            || schema["maximum"].as_f64().is_some_and(|m| n > m))
    {
        issue(
            out,
            p,
            "range",
            "number is outside its supported range",
            v,
            Severity::Error,
        );
    }
    if let Some(s) = v.as_str() {
        let bad = schema["minLength"]
            .as_u64()
            .is_some_and(|n| s.chars().count() < (n as usize))
            || schema["pattern"]
                .as_str()
                .is_some_and(|p| regex::Regex::new(p).is_ok_and(|r| !r.is_match(s)));
        let bad_format = match schema["format"].as_str() {
            Some("theme-color") => Rgba::parse(s).is_none(),
            Some("theme-asset") => !safe_asset(s),
            Some("theme-sound") => {
                !safe_asset(s)
                    || s.contains('/')
                    || s.contains("..")
                    || ![".wav", ".ogg", ".mp3", ".m4a"]
                        .iter()
                        .any(|ext| s.to_ascii_lowercase().ends_with(ext))
            }
            _ => false,
        };
        if bad || bad_format {
            issue(
                out,
                p,
                "string",
                "invalid string, color, reference or asset path",
                v,
                Severity::Error,
            );
        }
    }
    if let Some(a) = v.as_array() {
        if schema["minItems"]
            .as_u64()
            .is_some_and(|n| a.len() < (n as usize))
            || schema["maxItems"]
                .as_u64()
                .is_some_and(|n| a.len() > (n as usize))
        {
            issue(
                out,
                p,
                "length",
                "array has unsupported length",
                v,
                Severity::Error,
            );
        }
        for (i, x) in a.iter().enumerate() {
            if let Some(s) = schema["prefixItems"].get(i).or_else(|| schema.get("items")) {
                check(x, s, &path(p, &i.to_string()), out, depth + 1);
            }
        }
    }
    if let Some(o) = v.as_object() {
        if let Some(required) = schema["required"].as_array() {
            for k in required.iter().filter_map(Value::as_str) {
                if !o.contains_key(k) {
                    issue(
                        out,
                        &path(p, k),
                        "required",
                        "required property is missing",
                        &Value::Null,
                        Severity::Error,
                    );
                }
            }
        }
        for (k, x) in o {
            if let Some(s) = schema["properties"].get(k) {
                check(x, s, &path(p, k), out, depth + 1);
            } else {
                if schema["x-unknown-warning"] == true {
                    issue(
                        out,
                        &path(p, k),
                        "unknown",
                        "unknown key is retained but not interpreted",
                        x,
                        Severity::Warning,
                    );
                }
                if let Some(s) = schema.get("additionalProperties") {
                    check(x, s, &path(p, k), out, depth + 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn enum_and_const_compare_json_numbers_recursively_without_integer_rounding() {
        for (expected, actual, equal) in [
            (json!(2), json!(2.0), true),
            (json!(0), json!(-0.0), true),
            (json!(-2), json!(-2.0), true),
            (json!(2), json!(2.0000000000000004), false),
            (json!(2), json!("2"), false),
            (json!(1), json!(true), false),
            (
                json!(9_007_199_254_740_993u64),
                json!(9_007_199_254_740_992.0),
                false,
            ),
            (json!(i64::MAX), json!(i64::MAX as f64), false),
            (json!(i64::MIN), json!(i64::MIN as f64), true),
            (json!(u64::MAX), json!(u64::MAX as f64), false),
            (json!(1u64 << 63), json!((1u64 << 63) as f64), true),
            (json!({"items":[2,0]}), json!({"items":[2.0,-0.0]}), true),
        ] {
            for schema in [json!({"enum":[expected]}), json!({"const":expected})] {
                let mut diagnostics = vec![];
                check(&actual, &schema, "/probe", &mut diagnostics, 0);
                assert_eq!(diagnostics.is_empty(), equal, "{schema}: {actual}");
                assert!(
                    diagnostics
                        .iter()
                        .all(|d| d.location == "/probe" && d.code == "enum")
                );
            }
        }
    }
}
