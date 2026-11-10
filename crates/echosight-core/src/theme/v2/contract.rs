//! The shipped authoring schema is also the structural validator's source of truth.
use super::{Diagnostic, Severity, color::Rgba};
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
    out.push(Diagnostic {
        severity,
        location: location.into(),
        code: code.into(),
        message: message.into(),
        value: value.clone(),
    });
}
pub(super) fn path(parent: &str, key: &str) -> String {
    format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"))
}
pub(super) fn safe_asset(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(['\\', ':', '\0'])
        && !s.starts_with('/')
        && s.split('/').all(|p| !matches!(p, "" | "." | ".."))
}
pub(super) fn check(v: &Value, schema: &Value, p: &str, out: &mut Vec<Diagnostic>, depth: usize) {
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
                out.extend(d);
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
    if schema.get("const").is_some_and(|c| c != v)
        || schema["enum"].as_array().is_some_and(|a| !a.contains(v))
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
