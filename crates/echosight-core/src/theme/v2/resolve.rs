use super::{
    Diagnostic, Severity,
    contract::{issue, path},
};
use serde_json::{Value, json};

/// Check before cloning diagnostics or expanding references, including unknown fields.
pub(super) fn bounded_input(value: &Value, d: &mut Vec<Diagnostic>) -> bool {
    let mut pending = vec![(value, String::new(), 0usize)];
    let mut visits = 0usize;
    while let Some((value, p, depth)) = pending.pop() {
        visits += 1;
        if depth > 64 || visits > 100_000 {
            issue(
                d,
                &p,
                "budget",
                "input exceeds 64 levels or 100000 nodes; source value omitted for safety",
                &Value::Null,
                Severity::Error,
            );
            return false;
        }
        match value {
            Value::Object(o) => pending.extend(o.iter().map(|(k, v)| (v, path(&p, k), depth + 1))),
            Value::Array(a) => pending.extend(
                a.iter()
                    .enumerate()
                    .map(|(i, v)| (v, path(&p, &i.to_string()), depth + 1)),
            ),
            _ => {}
        }
    }
    true
}

pub(super) fn token_path(reference: &str) -> String {
    let p = reference
        .trim_start_matches('@')
        .strip_prefix("tokens.")
        .unwrap_or(reference.trim_start_matches('@'));
    if p.starts_with("surface.") || p.starts_with("text.") || p.starts_with("border.") {
        format!("colors.{p}")
    } else {
        p.to_string()
    }
}
fn lookup<'a>(tokens: &'a Value, key: &str) -> Option<&'a Value> {
    let mut v = tokens;
    for segment in key.split('.') {
        v = v.get(segment)?;
    }
    Some(v)
}
pub(super) fn resolve_refs(
    v: &Value,
    tokens: &Value,
    p: &str,
    stack: &mut Vec<String>,
    d: &mut Vec<Diagnostic>,
    depth: usize,
) -> Option<Value> {
    resolve_bounded(v, tokens, p, stack, d, depth, &mut 100_000)
}
fn reference_position(p: &str) -> bool {
    fn allows(schema: &Value, parts: &[String], depth: usize) -> bool {
        if depth > 96 {
            return false;
        }
        if let Some(r) = schema["$ref"].as_str() {
            if parts.is_empty() && r == "#/$defs/reference" {
                return true;
            }
            return super::contract::CONTRACT
                .pointer(r.trim_start_matches('#'))
                .is_some_and(|s| allows(s, parts, depth + 1));
        }
        if let Some(options) = schema["anyOf"].as_array() {
            return options.iter().any(|s| allows(s, parts, depth + 1));
        }
        let Some((key, rest)) = parts.split_first() else {
            return false;
        };
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
        child.is_some_and(|s| allows(s, rest, depth + 1))
    }
    let parts: Vec<_> = p
        .trim_start_matches('/')
        .split('/')
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect();
    allows(&super::contract::CONTRACT, &parts, 0)
}
fn resolve_bounded(
    v: &Value,
    tokens: &Value,
    p: &str,
    stack: &mut Vec<String>,
    d: &mut Vec<Diagnostic>,
    depth: usize,
    budget: &mut usize,
) -> Option<Value> {
    if *budget == 0 {
        issue(
            d,
            p,
            "budget",
            "resolved document exceeds 100000 visits",
            &Value::Null,
            Severity::Error,
        );
        return None;
    }
    *budget -= 1;
    if depth > 64 {
        issue(
            d,
            p,
            "depth",
            "maximum reference/value depth is 64",
            v,
            Severity::Error,
        );
        return None;
    }
    if let Some(reference) = v
        .as_str()
        .filter(|s| s.starts_with('@') && reference_position(p))
    {
        let key = token_path(reference);
        if stack.contains(&key) {
            issue(
                d,
                p,
                "reference-cycle",
                &format!("cyclic token reference: {} -> {key}", stack.join(" -> ")),
                v,
                Severity::Error,
            );
            return None;
        }
        let Some(target) = lookup(tokens, &key) else {
            issue(
                d,
                p,
                "missing-reference",
                "token does not exist",
                v,
                Severity::Error,
            );
            return None;
        };
        stack.push(key);
        let result = resolve_bounded(target, tokens, p, stack, d, depth + 1, budget);
        stack.pop();
        return result;
    }
    match v {
        Value::Object(o) => {
            let mut result = serde_json::Map::new();
            let mut ok = true;
            for (k, v) in o {
                if let Some(v) =
                    resolve_bounded(v, tokens, &path(p, k), stack, d, depth + 1, budget)
                {
                    result.insert(k.clone(), v);
                } else {
                    ok = false;
                }
            }
            ok.then_some(Value::Object(result))
        }
        Value::Array(a) => {
            let mut result = vec![];
            let mut ok = true;
            for (i, v) in a.iter().enumerate() {
                if let Some(v) = resolve_bounded(
                    v,
                    tokens,
                    &path(p, &i.to_string()),
                    stack,
                    d,
                    depth + 1,
                    budget,
                ) {
                    result.push(v);
                } else {
                    ok = false;
                }
            }
            ok.then_some(Value::Array(result))
        }
        _ => Some(v.clone()),
    }
}
/// Styles replace arrays; only variants concatenate arrays (defu).
pub(super) fn merge_style(base: &mut Value, overrides: &Value) {
    if let (Some(b), Some(o)) = (base.as_object_mut(), overrides.as_object()) {
        for (k, v) in o {
            // Fill variants and image modes are tagged unions: do not leave stale geometry.
            if k == "fill" || (k == "image" && v.get("mode").is_some()) {
                b.insert(k.clone(), v.clone());
            } else if let Some(existing) = b.get_mut(k) {
                merge_style(existing, v);
            } else {
                b.insert(k.clone(), v.clone());
            }
        }
    } else {
        *base = overrides.clone();
    }
}
pub(super) fn expand_slot(
    name: &str,
    slots: &Value,
    stack: &mut Vec<String>,
    d: &mut Vec<Diagnostic>,
) -> Option<Value> {
    let p = path("/slots", name);
    if stack.len() > 64 || stack.iter().any(|s| s == name) {
        issue(
            d,
            &p,
            "inheritance-cycle",
            "cyclic or too-deep slot inheritance",
            &slots[name],
            Severity::Error,
        );
        return None;
    }
    let Some(slot) = slots.get(name) else {
        issue(
            d,
            &p,
            "missing-slot",
            "inherited slot does not exist",
            &json!(name),
            Severity::Error,
        );
        return None;
    };
    stack.push(name.to_string());
    let mut result = if let Some(parent) = slot["$extends"].as_str() {
        expand_slot(parent, slots, stack, d)?
    } else {
        json!({})
    };
    merge_style(&mut result, slot);
    if let Some(o) = result.as_object_mut() {
        o.shift_remove("$extends");
    }
    stack.pop();
    Some(result)
}
pub(super) fn semantics(v: &Value, p: &str, d: &mut Vec<Diagnostic>) {
    if let Some(o) = v.as_object() {
        if let Some(stops) = o.get("stops").and_then(Value::as_array) {
            let mut previous = 0.;
            for (i, stop) in stops.iter().enumerate() {
                if let Some(n) = stop.get(1).and_then(Value::as_f64) {
                    if n < previous {
                        issue(
                            d,
                            &path(&path(p, "stops"), &i.to_string()),
                            "stop-order",
                            "gradient stops must be nondecreasing",
                            stop,
                            Severity::Error,
                        );
                    }
                    previous = n;
                }
            }
        }
        if o.get("mode").and_then(Value::as_str) == Some("nineSlice") && !o.contains_key("insets") {
            issue(
                d,
                p,
                "insets",
                "nineSlice requires four insets",
                v,
                Severity::Error,
            );
        }
        for (k, v) in o {
            semantics(v, &path(p, k), d);
        }
    } else if let Some(a) = v.as_array() {
        for (i, v) in a.iter().enumerate() {
            semantics(v, &path(p, &i.to_string()), d);
        }
    }
}
pub(super) fn opacity_matches(slot: &str, fill: &Value, tokens: &Value) -> bool {
    let patterns: Vec<&str> = tokens["opacity"]["appliesTo"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_else(|| vec!["surface.*", "window.*", "templateSection", "tasksSection"]);
    fn matches(pattern: &str, key: &str) -> bool {
        pattern == key
            || pattern
                .strip_suffix('*')
                .is_some_and(|p| key.starts_with(p))
    }
    if patterns.iter().any(|p| matches(p, slot)) {
        return true;
    }
    fn visit(v: &Value, tokens: &Value, patterns: &[&str], stack: &mut Vec<String>) -> bool {
        if stack.len() > 64 {
            return false;
        }
        if let Some(reference) = v.as_str().filter(|s| s.starts_with('@')) {
            let key = token_path(reference);
            if patterns.iter().any(|p| {
                matches(p, &key) || matches(p, key.strip_prefix("colors.").unwrap_or(&key))
            }) {
                return true;
            }
            if stack.contains(&key) {
                return false;
            }
            stack.push(key.clone());
            let result = lookup(tokens, &key).is_some_and(|v| visit(v, tokens, patterns, stack));
            stack.pop();
            return result;
        }
        match v {
            Value::Object(o) => o.values().any(|v| visit(v, tokens, patterns, stack)),
            Value::Array(a) => a.iter().any(|v| visit(v, tokens, patterns, stack)),
            _ => false,
        }
    }
    visit(fill, tokens, &patterns, &mut vec![])
}
