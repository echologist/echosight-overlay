use super::{
    Diagnostic, Severity,
    contract::{CONTRACT, check, child_schema, issue, path, reference_position, selected_schema},
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
    schema: &Value,
    d: &mut Vec<Diagnostic>,
) -> Option<Value> {
    ReferenceResolver {
        tokens,
        diagnostics: d,
        budget: 100_000,
    }
    .resolve(v, schema, p, &mut vec![], 0)
}

struct ReferenceResolver<'a> {
    tokens: &'a Value,
    diagnostics: &'a mut Vec<Diagnostic>,
    budget: usize,
}
impl ReferenceResolver<'_> {
    fn resolve(
        &mut self,
        v: &Value,
        schema: &Value,
        p: &str,
        stack: &mut Vec<String>,
        depth: usize,
    ) -> Option<Value> {
        if self.budget == 0 {
            issue(
                self.diagnostics,
                p,
                "budget",
                "resolved document exceeds 100000 visits",
                &Value::Null,
                Severity::Error,
            );
            return None;
        }
        self.budget -= 1;
        if depth > 64 {
            issue(
                self.diagnostics,
                p,
                "depth",
                "maximum reference/value depth is 64",
                v,
                Severity::Error,
            );
            return None;
        }
        let selected = selected_schema(v, schema);
        if selected.is_none() && schema != &Value::Bool(true) {
            // Expanded references still have to satisfy their destination shape.
            check(v, schema, p, self.diagnostics, 0);
            return None;
        }
        if let Some(reference) = v
            .as_str()
            .filter(|_| selected.is_some_and(reference_position))
        {
            let key = token_path(reference);
            if stack.contains(&key) {
                issue(
                    self.diagnostics,
                    p,
                    "reference-cycle",
                    &format!("cyclic token reference: {} -> {key}", stack.join(" -> ")),
                    v,
                    Severity::Error,
                );
                return None;
            }
            let Some(target) = lookup(self.tokens, &key) else {
                issue(
                    self.diagnostics,
                    p,
                    "missing-reference",
                    "token does not exist",
                    v,
                    Severity::Error,
                );
                return None;
            };
            stack.push(key);
            // Keep the destination schema, not the token source's schema.
            let result = self.resolve(target, schema, p, stack, depth + 1);
            stack.pop();
            return result;
        }
        match v {
            Value::Object(o) => {
                let mut result = serde_json::Map::new();
                let mut ok = true;
                for (k, v) in o {
                    // Opaque extensions are copied without interpretation, but still
                    // charged to the expansion budget (including token-sourced data).
                    let child = selected
                        .and_then(|s| child_schema(s, k))
                        .unwrap_or(&Value::Bool(true));
                    let resolved = self.resolve(v, child, &path(p, k), stack, depth + 1);
                    if let Some(v) = resolved {
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
                    let key = i.to_string();
                    let child = selected
                        .and_then(|s| child_schema(s, &key))
                        .unwrap_or(&Value::Bool(true));
                    let resolved = self.resolve(v, child, &path(p, &key), stack, depth + 1);
                    if let Some(v) = resolved {
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
pub(super) fn semantics(v: &Value, schema: &Value, p: &str, d: &mut Vec<Diagnostic>) {
    let Some(schema) = selected_schema(v, schema) else {
        return;
    };
    if let Some(o) = v.as_object() {
        if schema["properties"].get("stops").is_some()
            && let Some(stops) = o.get("stops").and_then(Value::as_array)
        {
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
        if schema["properties"].get("insets").is_some()
            && o.get("mode").and_then(Value::as_str) == Some("nineSlice")
            && !o.contains_key("insets")
        {
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
            if let Some(child) = child_schema(schema, k) {
                semantics(v, child, &path(p, k), d);
            }
        }
    } else if let Some(a) = v.as_array() {
        for (i, v) in a.iter().enumerate() {
            let key = i.to_string();
            if let Some(child) = child_schema(schema, &key) {
                semantics(v, child, &path(p, &key), d);
            }
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
    fn visit(
        v: &Value,
        schema: &Value,
        tokens: &Value,
        patterns: &[&str],
        stack: &mut Vec<String>,
    ) -> bool {
        if stack.len() > 64 {
            return false;
        }
        let Some(selected) = selected_schema(v, schema) else {
            return false;
        };
        if let Some(reference) = v.as_str().filter(|_| reference_position(selected)) {
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
            let result =
                lookup(tokens, &key).is_some_and(|v| visit(v, schema, tokens, patterns, stack));
            stack.pop();
            return result;
        }
        match v {
            Value::Object(o) => o.iter().any(|(k, v)| {
                child_schema(selected, k).is_some_and(|s| visit(v, s, tokens, patterns, stack))
            }),
            Value::Array(a) => a.iter().enumerate().any(|(i, v)| {
                child_schema(selected, &i.to_string())
                    .is_some_and(|s| visit(v, s, tokens, patterns, stack))
            }),
            _ => false,
        }
    }
    visit(
        fill,
        &CONTRACT["$defs"]["fill"],
        tokens,
        &patterns,
        &mut vec![],
    )
}
