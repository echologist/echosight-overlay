//! CSS-free theme data. No filesystem, renderer, CSS execution or format dispatch side effects.
mod color;
mod convert;
mod css;
pub use convert::{Conversion, ConversionInput, convert_v1};
mod contract;
mod resolve;
mod resolved;
pub use color::Rgba;
pub use contract::SCHEMA;
use contract::{CONTRACT, check, issue, path};
use resolve::{expand_slot, resolve_refs};
pub use resolved::{
    Bevel, Border, Fill, GradientStop, ImageStyle, ResolvedStyle, Shadow, TextStyle,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub location: String,
    pub code: String,
    pub message: String,
    pub value: Value,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeFormat {
    V1,
    V2,
}
pub fn detect_format(v: &Value) -> Result<ThemeFormat, Vec<Diagnostic>> {
    if v.is_object() {
        match v.get("format") {
            None => return Ok(ThemeFormat::V1),
            Some(x) if contract::json_equal(x, &json!(1)) => return Ok(ThemeFormat::V1),
            Some(x) if contract::json_equal(x, &json!(2)) => return Ok(ThemeFormat::V2),
            _ => {}
        }
    }
    let mut d = vec![];
    issue(
        &mut d,
        "/format",
        "format",
        "expected absent/1 (v1) or 2 (v2)",
        &v["format"],
        Severity::Error,
    );
    Err(d)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Completed,
    Checked,
    Hover,
    Focus,
    Active,
    Disabled,
}
impl State {
    pub const PRECEDENCE: [Self; 6] = [
        Self::Completed,
        Self::Checked,
        Self::Hover,
        Self::Focus,
        Self::Active,
        Self::Disabled,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Checked => "checked",
            Self::Hover => "hover",
            Self::Focus => "focus",
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Theme {
    document: Value,
    expanded: Value,
    diagnostics: Vec<Diagnostic>,
}
impl Theme {
    pub fn parse(value: &Value) -> Result<Self, Vec<Diagnostic>> {
        Self::parse_depth(value, 0)
    }
    fn parse_depth(value: &Value, depth: usize) -> Result<Self, Vec<Diagnostic>> {
        let mut d = vec![];
        if !resolve::bounded_input(value, &mut d) {
            return Err(d);
        }
        check(value, &CONTRACT, "", &mut d, 0);
        if depth > 16 {
            issue(
                &mut d,
                "/variants",
                "depth",
                "variant depth exceeds 16",
                value,
                Severity::Error,
            );
        }
        if d.iter().any(|d| d.severity == Severity::Error) {
            sort(&mut d);
            return Err(d);
        }
        let mut expanded = json!({});
        let mut slots = value["slots"].clone();
        if !slots.is_object() {
            slots = json!({});
        }
        // Explicit slot images win over image mappings. Mapped states are defaults too.
        if let Some(images) = value["images"].as_object() {
            for (name, image) in images {
                if !slots[name].is_object() {
                    slots[name] = json!({});
                }
                let mut base = image.clone();
                if let Some(o) = base.as_object_mut() {
                    o.shift_remove("states");
                }
                if slots[name].get("image").is_none() {
                    slots[name]["image"] = base;
                }
                if let Some(states) = image["states"].as_object() {
                    for (state, image) in states {
                        if !slots[name]["states"].is_object() {
                            slots[name]["states"] = json!({});
                        }
                        if !slots[name]["states"][state].is_object() {
                            slots[name]["states"][state] = json!({});
                        }
                        if slots[name]["states"][state].get("image").is_none() {
                            slots[name]["states"][state]["image"] = image.clone();
                        }
                    }
                }
            }
        }
        if let Some(slots_map) = slots.as_object() {
            for name in slots_map.keys() {
                if let Some(s) = expand_slot(name, &slots, &mut vec![], &mut d) {
                    expanded[name] = s;
                }
            }
        }
        let mut check_value = value.clone();
        check_value["slots"] = expanded.clone();
        check_value
            .as_object_mut()
            .expect("checked object")
            .shift_remove("variants");
        if let Some(resolved) = resolve_refs(&check_value, &value["tokens"], "", &CONTRACT, &mut d)
        {
            check(&resolved, &CONTRACT, "", &mut d, 0);
            resolve::semantics(&resolved, &CONTRACT, "", &mut d);
        }
        let mut theme = Self {
            document: value.clone(),
            expanded,
            diagnostics: vec![],
        };
        if let Some(variants) = value["variants"].as_object() {
            for id in variants.keys() {
                let merged = theme.variant_document(id);
                if let Some(merged) = merged {
                    match Self::parse_depth(&merged, depth + 1) {
                        Ok(v) => d.extend(v.diagnostics.into_iter().map(|mut d| {
                            d.location = format!("{}{}", path("/variants", id), d.location);
                            d
                        })),
                        Err(errors) => d.extend(errors.into_iter().map(|mut d| {
                            d.location = format!("{}{}", path("/variants", id), d.location);
                            d
                        })),
                    }
                }
            }
        }
        sort(&mut d);
        if d.iter().any(|d| d.severity == Severity::Error) {
            Err(d)
        } else {
            theme.diagnostics = d;
            Ok(theme)
        }
    }
    pub fn document(&self) -> &Value {
        &self.document
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    pub fn slot_names(&self) -> impl Iterator<Item = &str> {
        self.expanded
            .as_object()
            .into_iter()
            .flat_map(|m| m.keys().map(String::as_str))
    }
    pub fn slot(&self, name: &str, states: &[State], user_opacity: f64) -> Option<ResolvedStyle> {
        let mut raw = self.expanded.get(name)?.clone();
        for state in State::PRECEDENCE {
            if states.contains(&state)
                && let Some(overrides) = self.expanded[name]["states"].get(state.as_str())
            {
                resolve::merge_style(&mut raw, overrides);
            }
        }
        raw.as_object_mut()?.shift_remove("states");
        raw.as_object_mut()?.shift_remove("$extends");
        let affects = raw["opacityAffects"].as_bool().unwrap_or_else(|| {
            resolve::opacity_matches(name, &raw["fill"], &self.document["tokens"])
        });
        let mut resolved = resolve_refs(
            &raw,
            &self.document["tokens"],
            &path("/slots", name),
            &CONTRACT["$defs"]["slot"],
            &mut vec![],
        )?;
        if affects {
            let user = if user_opacity.is_finite() {
                user_opacity.clamp(0., 1.)
            } else {
                1.
            };
            let mode = match name {
                "window.interactive" => "interactive",
                "window.clickThrough" => "clickThrough",
                _ => "",
            };
            let mode_opacity = resolve_refs(
                &self.document["tokens"]["opacity"][mode],
                &self.document["tokens"],
                &path("/tokens/opacity", mode),
                &CONTRACT["$defs"]["alpha"],
                &mut vec![],
            )
            .and_then(|v| v.as_f64())
            .unwrap_or(1.);
            let factor = user * mode_opacity;
            if let Some(fill) = resolved.get_mut("fill") {
                if fill.is_string() {
                    *fill = json!({"color":fill.clone(),"alpha":factor});
                } else if fill.is_object() {
                    fill["alpha"] = json!(fill["alpha"].as_f64().unwrap_or(1.) * factor);
                }
            }
        }
        Some(ResolvedStyle::new(resolved))
    }
    fn variant_document(&self, id: &str) -> Option<Value> {
        let patch = self.document["variants"].get(id)?.as_object()?;
        let mut base = self.document.as_object()?.clone();
        base.shift_remove("variants");
        let mut result = super::helpers::deep_merge_defu(patch, &base);
        result.shift_remove("variants");
        result.insert("format".into(), json!(2));
        result.insert(
            "id".into(),
            json!(format!("{}-{id}", self.document["id"].as_str()?)),
        );
        if !patch
            .get("name")
            .is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()))
        {
            result.insert(
                "name".into(),
                json!(format!("{} ({id})", self.document["name"].as_str()?)),
            );
        }
        Some(Value::Object(result))
    }
    pub fn variant(&self, id: &str) -> Result<Self, Vec<Diagnostic>> {
        if let Some(v) = self.variant_document(id) {
            Self::parse(&v)
        } else {
            let mut d = vec![];
            issue(
                &mut d,
                &path("/variants", id),
                "missing-variant",
                "variant does not exist",
                &json!(id),
                Severity::Error,
            );
            Err(d)
        }
    }
}
pub(super) fn sort(d: &mut Vec<Diagnostic>) {
    d.sort_by(|a, b| {
        (&a.location, &a.code, &a.message, a.severity).cmp(&(
            &b.location,
            &b.code,
            &b.message,
            b.severity,
        ))
    });
    d.dedup();
}
