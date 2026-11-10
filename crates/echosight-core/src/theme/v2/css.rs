//! Deliberately bounded CSS value translation and tokenizer, never a CSS engine.
use super::{Rgba, resolve::token_path};
use serde_json::{Value, json};

/// Split only outside parentheses/quotes. Preserve empty list items, but coalesce
/// whitespace separators for lengths. Declaration callers ignore empty trivia.
pub(super) fn split_top(s: &str, delimiter: char) -> Vec<&str> {
    let mut out = vec![];
    let mut start = 0;
    let mut depth = 0usize;
    let mut quote = None;
    let mut escape = false;
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            continue;
        }
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && (c == delimiter || (delimiter == ' ' && c.is_whitespace())) {
            if delimiter != ' ' || start < i {
                out.push(s[start..i].trim());
            }
            start = i + c.len_utf8();
        }
    }
    if delimiter != ' ' || start < s.len() {
        out.push(s[start..].trim());
    }
    out
}
pub(super) fn length(s: &str) -> Option<f64> {
    let n = s
        .trim()
        .strip_suffix("px")
        .unwrap_or(s.trim())
        .parse::<f64>()
        .ok()?;
    n.is_finite().then_some(n)
}
pub(super) fn edges(s: &str, negative: bool, kind: &str) -> Option<Value> {
    let a: Vec<f64> = split_top(s, ' ')
        .iter()
        .map(|x| length(x))
        .collect::<Option<_>>()?;
    if !negative && a.iter().any(|x| *x < 0.) {
        return None;
    }
    match a.as_slice() {
        [x] => Some(json!(x)),
        [a, b] if kind != "radius" => Some(json!([a, b])),
        [a, b] => Some(json!([a, b, a, b])),
        [a, b, c] => Some(json!([a, b, c, b])),
        [a, b, c, d] => Some(json!([a, b, c, d])),
        _ => None,
    }
}
pub(super) fn family(s: &str) -> Value {
    json!(
        split_top(s, ',')
            .iter()
            .map(|s| s.trim_matches(['\'', '"']).to_string())
            .collect::<Vec<_>>()
    )
}
pub(super) fn lookup<'a>(tokens: &'a Value, key: &str) -> Option<&'a Value> {
    let mut v = tokens;
    for p in token_path(key).split('.') {
        v = v.get(p)?;
    }
    Some(v)
}
fn variable(s: &str, tokens: &Value) -> Option<Value> {
    let inner = s.strip_prefix("var(")?.strip_suffix(')')?;
    let parts = split_top(inner, ',');
    let key = parts.first()?.trim().strip_prefix("--")?;
    let path = if let Some(k) = key.strip_prefix("bg-") {
        format!("colors.surface.{k}")
    } else if let Some(k) = key.strip_prefix("text-") {
        format!("colors.text.{k}")
    } else if let Some(k) = key.strip_prefix("border-") {
        let alias = match k {
            "light" if tokens["colors"]["border"].get("light").is_none() => "primary",
            "dark" if tokens["colors"]["border"].get("dark").is_none() => "secondary",
            _ => k,
        };
        format!("colors.border.{alias}")
    } else {
        format!("colors.{key}")
    };
    if lookup(tokens, &path).is_some() {
        Some(json!(format!("@{path}")))
    } else {
        parts.get(1).and_then(|v| color(v, tokens))
    }
}
pub(super) fn color(s: &str, tokens: &Value) -> Option<Value> {
    let s = s.trim();
    if let Some(v) = variable(s, tokens) {
        return Some(v);
    }
    if Rgba::parse(s).is_some() {
        return Some(json!(s));
    }
    legacy_alpha(s).and_then(|s| Rgba::parse(&s).map(|_| json!(s)))
}
/// Only the shipped user-opacity idiom; arbitrary calc/var expressions are losses.
fn legacy_alpha(s: &str) -> Option<String> {
    let inner = s.strip_prefix("rgba(")?.strip_suffix(')')?;
    let parts = split_top(inner, ',');
    if parts.len() != 4 {
        return None;
    }
    let alpha = parts[3];
    let re = regex::Regex::new(r"var\(\s*--user-transparency\s*(?:,\s*[0-9.]+\s*)?\)").ok()?;
    if !re.is_match(alpha) {
        return None;
    }
    let replaced = re.replace_all(alpha, "1");
    let x = replaced
        .trim()
        .strip_prefix("calc(")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(replaced.trim());
    let factors: Vec<f64> = x
        .split('*')
        .map(|n| n.trim().parse::<f64>().ok())
        .collect::<Option<_>>()?;
    if factors.is_empty() || factors.len() > 2 {
        return None;
    }
    Some(format!(
        "rgba({}, {}, {}, {})",
        parts[0],
        parts[1],
        parts[2],
        factors.iter().product::<f64>()
    ))
}
pub(super) fn fill(s: &str, tokens: &Value) -> Option<Value> {
    if let Some(c) = color(s, tokens) {
        return Some(c);
    }
    let inner = s.strip_prefix("linear-gradient(")?.strip_suffix(')')?;
    let mut parts = split_top(inner, ',');
    if parts.len() < 2 || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    let angle = match parts[0] {
        "to bottom" => Some(180.),
        "to top" => Some(0.),
        "to right" => Some(90.),
        "to left" => Some(270.),
        v => v.strip_suffix("deg").and_then(length),
    };
    let angle = if let Some(a) = angle {
        parts.remove(0);
        a
    } else {
        180.
    };
    if parts.len() < 2 {
        return None;
    }
    let mut stops: Vec<(Value, Option<f64>)> = vec![];
    for part in parts {
        let (source, pos) = gradient_stop(part);
        stops.push((color(source, tokens)?, pos));
    }
    let last = stops.len() - 1;
    if stops[0].1.is_none() {
        stops[0].1 = Some(0.);
    }
    if stops[last].1.is_none() {
        stops[last].1 = Some(1.);
    }
    let mut left = 0;
    while left < last {
        let right = (left + 1..=last).find(|i| stops[*i].1.is_some())?;
        let a = stops[left].1?;
        let b = stops[right].1?;
        if !(0.0..=1.).contains(&a) || !(a..=1.).contains(&b) {
            return None;
        }
        for (i, stop) in stops.iter_mut().enumerate().take(right).skip(left + 1) {
            stop.1 = Some(a + (b - a) * (i - left) as f64 / (right - left) as f64);
        }
        left = right;
    }
    Some(
        json!({"gradient":{"angle":angle,"stops":stops.into_iter().map(|(c,p)|json!([c,p])).collect::<Vec<_>>()}}),
    )
}
fn gradient_stop(part: &str) -> (&str, Option<f64>) {
    if let Some(field) = split_top(part, ' ').last()
        && let Some(pos) = field.strip_suffix('%').and_then(length)
    {
        let end = part.len() - field.len();
        (part[..end].trim(), Some(pos / 100.))
    } else {
        (part, None)
    }
}

/// Classify the selected source color, not text in an unused var() fallback.
fn color_opacity(s: &str, tokens: &Value) -> bool {
    if let Some(v) = variable(s, tokens) {
        if v.as_str().is_some_and(|v| v.starts_with('@')) {
            return super::resolve::opacity_matches("", &v, tokens);
        }
        return s
            .strip_prefix("var(")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|inner| split_top(inner, ',').get(1).copied())
            .is_some_and(|fallback| color_opacity(fallback, tokens));
    }
    legacy_alpha(s).is_some()
}

/// One flag per source stop (or one for a solid), for an already accepted fill.
/// The v2 fill alpha cannot express a mixture of these flags.
pub(super) fn fill_opacity(s: &str, fill: &Value, tokens: &Value) -> Vec<bool> {
    if let Some(stops) = fill["gradient"]["stops"].as_array()
        && let Some(inner) = s
            .strip_prefix("linear-gradient(")
            .and_then(|s| s.strip_suffix(')'))
    {
        let parts = split_top(inner, ',');
        return parts
            .iter()
            .skip(parts.len() - stops.len())
            .map(|part| color_opacity(gradient_stop(part).0, tokens))
            .collect();
    }
    vec![color_opacity(s, tokens)]
}

pub(super) fn shadows(s: &str, tokens: &Value) -> Option<Value> {
    if s == "none" {
        return Some(json!([]));
    }
    let mut result = vec![];
    for part in split_top(s, ',') {
        let words = split_top(part, ' ');
        if words.contains(&"inset") {
            return None;
        }
        let mut nums = vec![];
        let mut c = None;
        for word in words {
            if let Some(n) = length(word) {
                nums.push(n);
            } else if c.is_none() {
                c = Some(color(word, tokens)?);
            } else {
                return None;
            }
        }
        if !(2..=4).contains(&nums.len()) || nums.get(2).is_some_and(|v| *v < 0.) {
            return None;
        }
        result.push(json!({"x":nums[0],"y":nums[1],"blur":nums.get(2).copied().unwrap_or(0.),"spread":nums.get(3).copied().unwrap_or(0.),"color":c?}));
    }
    (!result.is_empty()).then(|| json!(result))
}
pub(super) fn border(s: &str, tokens: &Value) -> Option<Value> {
    if s == "none" {
        return Some(json!({"width":0,"style":"none"}));
    }
    let parts = split_top(s, ' ');
    if parts.len() != 3 {
        return None;
    }
    let width = length(parts[0])?;
    if width < 0. || !matches!(parts[1], "solid" | "dashed" | "dotted") {
        return None;
    }
    Some(json!({"width":width,"style":parts[1],"color":color(parts[2],tokens)?}))
}

#[derive(Debug)]
pub(super) enum Rule {
    Style { selector: String, body: String },
    Unsupported(String),
}
/// Comments are discarded as trivia. At-rules, nested rules and malformed tails are reported whole.
pub(super) fn rules(css: &str) -> Vec<Rule> {
    let mut cleaned = String::new();
    let mut chars = css.chars().peekable();
    let mut quote = None;
    let mut escape = false;
    while let Some(c) = chars.next() {
        if escape {
            cleaned.push(c);
            escape = false;
            continue;
        }
        if c == '\\' {
            cleaned.push(c);
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            cleaned.push(c);
            if c == q {
                quote = None;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            cleaned.push(c);
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    closed = true;
                    break;
                }
            }
            if !closed {
                return vec![Rule::Unsupported(css.into())];
            }
            cleaned.push(' ');
        } else {
            cleaned.push(c);
        }
    }
    let mut result = vec![];
    let mut start = 0;
    let mut open = None;
    let mut depth = 0usize;
    let mut nested = false;
    let mut quote = None;
    let mut escape = false;
    for (i, c) in cleaned.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            continue;
        }
        match c {
            '{' => {
                if depth == 0 {
                    open = Some(i);
                } else {
                    nested = true;
                }
                depth += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let j = open.unwrap_or(start);
                    let selector = cleaned[start..j].trim();
                    let whole = cleaned[start..=i].trim();
                    if nested || selector.starts_with('@') {
                        result.push(Rule::Unsupported(whole.into()));
                    } else {
                        result.push(Rule::Style {
                            selector: selector.into(),
                            body: cleaned[j + 1..i].into(),
                        });
                    }
                    start = i + 1;
                    nested = false;
                }
            }
            ';' if depth == 0 => {
                let s = cleaned[start..=i].trim();
                if !s.is_empty() {
                    result.push(Rule::Unsupported(s.into()));
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    if !cleaned[start..].trim().is_empty() {
        result.push(Rule::Unsupported(cleaned[start..].trim().into()));
    }
    result
}

pub(super) fn selector(s: &str) -> Option<(Vec<&'static str>, Option<&'static str>)> {
    let mut s = s.trim();
    let mut state = None;
    for (suffix, name) in [
        (":hover", "hover"),
        (":active", "active"),
        (":focus", "focus"),
        (":disabled", "disabled"),
        (":checked", "checked"),
        (".completed", "completed"),
    ] {
        if let Some(base) = s.strip_suffix(suffix) {
            s = base;
            state = Some(name);
            break;
        }
    }
    if s == ".overlay-container.interactive" {
        return Some((vec!["window.interactive"], state));
    }
    if s == ".overlay-container.click-through" {
        return Some((vec!["window.clickThrough"], state));
    }
    s = s
        .strip_prefix(".overlay-container.interactive ")
        .unwrap_or(s);
    let names = match s {
        ".header" => vec!["header"],
        ".header h2" => vec!["headerTitle"],
        ".template-section" => vec!["templateSection"],
        ".tasks-section" => vec!["tasksSection"],
        ".section-title" => vec!["sectionTitle"],
        ".task-item" => vec!["taskRow"],
        ".task-text" => vec!["taskText"],
        ".task-checkbox" => vec!["checkbox"],
        ".progress-bar" => vec!["progressBar"],
        ".progress-fill" => vec!["progressFill"],
        ".progress-text" => vec!["progressText"],
        "button" | ".template-btn" | ".add-btn" => vec!["button"],
        ".header-btn" => vec!["button.icon"],
        "input" => vec!["input"],
        "textarea" => vec!["textarea"],
        "select" => vec!["select"],
        ".context-menu" => vec!["contextMenu"],
        ".context-menu-item" => vec!["contextMenuItem"],
        ".modal" => vec!["dialog.scrim"],
        ".modal-content" => vec!["dialog.window"],
        ".modal-title" | ".settings-title" | ".modal-content h4" => vec!["dialog.title"],
        ".app-dialog-message" | ".trigger-description" | ".community-template .description" => {
            vec!["dialog.text"]
        }
        ".import-hint"
        | ".settings-help"
        | ".hotkey-help"
        | ".range-labels"
        | ".community-template .task-count" => vec!["dialog.hint"],
        ".community-template" => vec!["communityTemplateCard"],
        ".modal-btn" => vec!["dialog.button.primary", "dialog.button.secondary"],
        ".modal-btn.primary" => vec!["dialog.button.primary"],
        ".modal-btn.secondary" => vec!["dialog.button.secondary"],
        ".modal-content-dialog.app-dialog-tone-danger .modal-title" => vec!["dialog.danger.title"],
        _ => return None,
    };
    Some((names, state))
}
