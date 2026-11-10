//! Pure, deterministic, explicitly lossy v1 -> v2 conversion. No files are read or written.
use super::{
    Diagnostic, Severity, State, Theme, ThemeFormat,
    contract::{issue, path, safe_asset},
    css, detect_format,
    resolve::merge_style,
    sort,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Debug, Clone, Copy)]
pub struct ConversionInput<'a> {
    pub document: &'a Value,
    pub css_file: Option<&'a str>,
    pub asset_files: &'a [String],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversion {
    pub theme: Value,
    pub diagnostics: Vec<Diagnostic>,
    pub provisional: bool,
}
impl Conversion {
    pub fn report_markdown(&self) -> String {
        let mut s = String::from(
            "# Theme conversion report\n\nStructural conversion only; not visual parity. Review every loss before publishing.\n",
        );
        if self.provisional {
            s.push_str("\n**Provisional CSS-folder theme: manual authoring and visual approval required.**\n");
        }
        for d in &self.diagnostics {
            s.push_str(&format!(
                "\n- {} at {}: {}\n\n    {}\n",
                d.code,
                serde_json::to_string(&d.location).unwrap_or_default(),
                d.message,
                serde_json::to_string(&d.value).unwrap_or_default()
            ));
        }
        s
    }
}
// Generated common CSS applies these effects to every button, including modal
// and header buttons. v2 slots are independent, not implicit DOM descendants.
const BUTTON_EFFECT_SLOTS: &[&str] = &[
    "button",
    "button.primary",
    "button.danger",
    "button.icon",
    "dialog.button.primary",
    "dialog.button.secondary",
];

struct Converter {
    out: Value,
    diagnostics: Vec<Diagnostic>,
}
impl Converter {
    fn loss(&mut self, p: &str, code: &str, message: &str, v: &Value) {
        issue(
            &mut self.diagnostics,
            p,
            code,
            message,
            v,
            Severity::Warning,
        );
    }
    fn slot(&mut self, name: &str, state: Option<&str>, style: &Value) {
        if !self.out["slots"][name].is_object() {
            self.out["slots"][name] = json!({});
        }
        let mut target = &mut self.out["slots"][name];
        if let Some(state) = state {
            if !target["states"].is_object() {
                target["states"] = json!({});
            }
            if !target["states"][state].is_object() {
                target["states"][state] = json!({});
            }
            target = &mut target["states"][state];
        }
        merge_style(target, style);
    }
    fn colors(&mut self, v: &Value, p: &str, key: &str) -> Option<Value> {
        if let Some(o) = v.as_object() {
            let mut result = serde_json::Map::new();
            for (k, v) in o {
                let target = if key.is_empty() && k == "background" {
                    "surface"
                } else {
                    k
                };
                let key = if key.is_empty() {
                    target.into()
                } else {
                    format!("{key}.{target}")
                };
                if let Some(v) = self.colors(v, &path(p, k), &key) {
                    result.insert(target.into(), v);
                }
            }
            return Some(Value::Object(result));
        }
        if let Some(s) = v.as_str()
            && let Some(c) = css::color(s, &json!({}))
        {
            if key.starts_with("surface.")
                && (s.trim().starts_with("rgb(") || s.trim().starts_with("rgba("))
            {
                self.out["tokens"]["opacity"]["appliesTo"]
                    .as_array_mut()
                    .expect("initialized")
                    .push(json!(key));
            }
            return Some(c);
        }
        self.loss(
            p,
            "unsupported-color",
            "unsupported color syntax or value; token omitted",
            v,
        );
        None
    }
    fn declarations(&mut self, props: &Value, p: &str, slot: &str, state: Option<&str>) -> Value {
        let base = &self.out["slots"][slot];
        let previous_fill = state
            .and_then(|state| base["states"][state].get("fill"))
            .or_else(|| base.get("fill"))
            .cloned()
            .unwrap_or(Value::Null);
        let mut style = json!({});
        let Some(props) = props.as_object() else {
            self.loss(p, "unsupported-style", "expected CSS property map", props);
            return style;
        };
        for (k, v) in props {
            let mut property = String::new();
            for c in k.chars() {
                if c.is_ascii_uppercase() {
                    property.push('-');
                    property.push(c.to_ascii_lowercase());
                } else {
                    property.push(c);
                }
            }
            let raw = v
                .as_str()
                .map(str::to_owned)
                .or_else(|| v.as_number().map(ToString::to_string));
            let Some(raw) = raw else {
                self.loss(
                    &path(p, k),
                    "unsupported-property",
                    "property is not a string or number",
                    v,
                );
                continue;
            };
            let value = raw
                .trim()
                .strip_suffix("!important")
                .unwrap_or(raw.trim())
                .trim();
            let tokens = &self.out["tokens"];
            let mapped: Option<Value> = match property.as_str() {
                "background" | "background-color" => {
                    let fill = if property == "background-color" {
                        css::color(value, tokens)
                    } else {
                        css::fill(value, tokens)
                    };
                    fill.map(|f| {
                        // Explicit booleans survive variant/state merging. A solid
                        // override clears a generated flag, while a surface-token
                        // reference still participates in the generated opacity list.
                        let affects = css::fill_opacity(value, &f, tokens).contains(&true);
                        json!({"fill":f,"opacityAffects":affects})
                    })
                }
                "color" => css::color(value, tokens).map(|c| json!({"text":{"color":c}})),
                "border" => css::border(value, tokens).map(|b| json!({"border":b})),
                "border-color" => css::color(value, tokens).map(|c| json!({"border":{"color":c}})),
                "border-width" => css::length(value)
                    .filter(|n| *n >= 0.)
                    .map(|n| json!({"border":{"width":n}})),
                "border-radius" => css::edges(value, false, "radius").map(|n| json!({"radius":n})),
                "padding" | "margin" => css::edges(value, property == "margin", "edges")
                    .map(|n| json!({property.clone():n})),
                "height" => css::length(value)
                    .filter(|n| *n >= 0.)
                    .map(|n| json!({"height":n})),
                "font-size" => css::length(value)
                    .filter(|n| *n >= 0.)
                    .map(|n| json!({"text":{"size":n}})),
                "font-weight" => {
                    let weight = match value {
                        "normal" => Some(400.),
                        "bold" => Some(700.),
                        v => css::length(v),
                    };
                    weight
                        .filter(|n| (1.0..=1000.).contains(n))
                        .map(|n| json!({"text":{"weight":n}}))
                }
                "font-family" => Some(json!({"text":{"family":css::family(value)}})),
                "text-transform"
                    if matches!(value, "none" | "uppercase" | "lowercase" | "capitalize") =>
                {
                    Some(json!({"text":{"transform":value}}))
                }
                "letter-spacing" => {
                    if value == "normal" {
                        Some(json!({"text":{"letterSpacing":0}}))
                    } else {
                        css::length(value).map(|n| json!({"text":{"letterSpacing":n}}))
                    }
                }
                "text-decoration" if matches!(value, "line-through" | "none") => {
                    Some(json!({"text":{"strikethrough":value=="line-through"}}))
                }
                "text-shadow" | "box-shadow" => {
                    let token = match value {
                        "var(--text-shadow)" => Some("text"),
                        "var(--box-shadow)" => Some("window"),
                        "var(--button-shadow)" => Some("button"),
                        "var(--glow)" => Some("glow"),
                        "var(--bevel-inset)" => Some("bevelInset"),
                        "var(--bevel-outset)" => Some("bevelOutset"),
                        _ => None,
                    };
                    let shadows = if let Some(token) = token {
                        tokens["shadows"]
                            .get(token)
                            .map(|_| json!(format!("@shadows.{token}")))
                    } else {
                        css::shadows(value, tokens)
                    };
                    shadows.map(|s| {
                        if property == "text-shadow" {
                            json!({"text":{"shadow":s}})
                        } else {
                            json!({"shadow":s})
                        }
                    })
                }
                "outline" => css::border(value, tokens).map(|b| json!({"ring":b})),
                "outline-offset" => css::length(value).map(|n| json!({"ring":{"inset":-n}})),
                "background-image" if value.starts_with("asset:") => value
                    .strip_prefix("asset:")
                    .and_then(|name| {
                        self.out["images"]
                            .as_object()?
                            .values()
                            .find(|v| {
                                v["asset"].as_str().is_some_and(|s| {
                                    s.rsplit_once('.').is_some_and(|(base, _)| base == name)
                                })
                            })
                            .cloned()
                    })
                    .map(|i| json!({"image":i})),
                _ => None,
            };
            if let Some(mapped) = mapped {
                if let Some(fill) = mapped.get("fill") {
                    let source_opacity = css::fill_opacity(value, fill, &self.out["tokens"]);
                    if source_opacity.contains(&true) && source_opacity.contains(&false) {
                        self.loss(
                            &path(p, k),
                            "mixed-gradient-opacity",
                            "v1 multiplies only selected stop alphas by user opacity; v2 multiplies every stop alpha by user opacity because fill opacity is shared; unaffected source stops will also fade",
                            &json!({"source":v,"sourceStopOpacityAffects":source_opacity,
                                "v2StopOpacityAffects":vec![true; source_opacity.len()],"convertedFill":fill}),
                        );
                    }
                }
                // CSS background-color changes only the underlay, not a preceding
                // background gradient. A v2 fill has one layer, so retain the image
                // and report the missing underlay rather than replacing the gradient.
                let retained_fill = style.get("fill").unwrap_or(&previous_fill);
                if property == "background-color" && retained_fill.get("gradient").is_some() {
                    self.loss(
                        &path(p, k),
                        "background-layer-loss",
                        "background-color under a gradient cannot be represented by a single v2 fill; gradient retained, color underlay omitted",
                        &json!({"backgroundColor":v,"retainedFill":retained_fill}),
                    );
                    continue;
                }
                merge_style(&mut style, &mapped);
                if property == "font-family" {
                    self.loss(
                        &path(p, k),
                        "font-availability",
                        "family names retained; no font file is bundled or downloaded",
                        v,
                    );
                }
            } else {
                self.loss(
                    &path(p, k),
                    "unsupported-property",
                    "CSS declaration not representable by this bounded converter",
                    &json!({k:v}),
                );
            }
        }
        style
    }
    fn css(&mut self, text: &str, p: &str) {
        if !text.trim().is_empty() {
            self.loss(p,"css-source-order","recognized selectors are applied in source order, not CSS specificity/importance; manual review required",&json!(p));
        }
        for (i, rule) in css::rules(text).into_iter().enumerate() {
            let p = path(p, &i.to_string());
            match rule {
                css::Rule::Unsupported(s) => self.loss(
                    &p,
                    "unsupported-rule",
                    "at-rule, nested rule or malformed CSS is not executed",
                    &json!(s),
                ),
                css::Rule::Style { selector, body } => {
                    let mut props = serde_json::Map::new();
                    for declaration in css::split_top(&body, ';') {
                        // Empty declarations and trailing whitespace are valid trivia.
                        // Comma-list empties remain significant to gradient parsing.
                        if declaration.trim().is_empty() {
                            continue;
                        }
                        if let Some((k, v)) = declaration.split_once(':') {
                            if let Some(previous) = props.shift_remove(k.trim()) {
                                self.loss(
                                    &p,
                                    "duplicate-declaration",
                                    "earlier declaration discarded; retained declarations use final-occurrence order; invalid-value fallback and !important precedence are not preserved",
                                    &json!({k.trim():previous}),
                                );
                            }
                            props.insert(k.trim().into(), json!(v.trim()));
                        } else {
                            self.loss(
                                &p,
                                "malformed-declaration",
                                "declaration has no colon",
                                &json!(declaration),
                            );
                        }
                    }
                    for selector in css::split_top(&selector, ',') {
                        let p = path(&p, selector);
                        if let Some((slots, state)) = css::selector(selector) {
                            for slot in slots {
                                let style = self.declarations(
                                    &Value::Object(props.clone()),
                                    &p,
                                    slot,
                                    state,
                                );
                                self.slot(slot, state, &style);
                            }
                        } else {
                            for (k, v) in &props {
                                self.loss(
                                    &path(&p, k),
                                    "unsupported-selector",
                                    "selector is not in the fixed selector-to-slot table",
                                    &json!({k:v}),
                                );
                            }
                            if props.is_empty() {
                                self.loss(
                                    &p,
                                    "unsupported-selector",
                                    "selector is not in the fixed table",
                                    &json!(body),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    fn state_order(&mut self, states: &[(&str, Value)], p: &str) {
        let rank = |name: &str| State::PRECEDENCE.iter().position(|s| s.as_str() == name);
        for (i, (later, later_style)) in states.iter().enumerate() {
            for (earlier, earlier_style) in &states[..i] {
                if rank(earlier) <= rank(later) {
                    continue;
                }
                let mut source_order = earlier_style.clone();
                merge_style(&mut source_order, later_style);
                let mut v2_order = later_style.clone();
                merge_style(&mut v2_order, earlier_style);
                if source_order != v2_order {
                    self.loss(
                        p,
                        "simultaneous-state-order",
                        "v1 JSON state insertion order and fixed v2 state precedence differ when these states are simultaneous; individual states retained",
                        &json!({"sourceOrder":[earlier,later],"v2Order":[later,earlier],"sourceResult":source_order,"v2Result":v2_order}),
                    );
                }
            }
        }
    }
    fn component_font_size(&mut self, slot: &str, props: &Value, p: &str, style: &mut Value) {
        // The appended font stylesheet wins equal-specificity plain button/select
        // selectors. Class selectors still win in v1 and cannot share this one slot.
        // Input's generated attribute selectors are more specific: do not reset it.
        if !matches!(slot, "button" | "select") {
            return;
        }
        let Some(ui) = self.out["tokens"]["fonts"]["sizes"].get("ui").cloned() else {
            return;
        };
        let Some(size) = style["text"].get("size") else {
            return;
        };
        if size == &ui {
            return;
        }
        let key = if props.get("fontSize").is_some() {
            "fontSize"
        } else {
            "font-size"
        };
        self.loss(
            &path(p, key),
            "font-selector-precedence",
            "v1 appended fonts.primary.sizes.ui wins on the plain element but component class selectors retain their font-size; v2 keeps the plain-element size, not both selector outcomes",
            &json!({"component":props[key],"ui":ui}),
        );
        style["text"]["size"] = json!("@fonts.sizes.ui");
    }
    fn fonts(&mut self, fonts: &Value) {
        if let Some(primary) = fonts.get("primary")
            && !primary.is_object()
        {
            self.loss(
                "/fonts/primary",
                "unsupported-font",
                "expected primary font object",
                primary,
            );
        }
        if let Some(o) = fonts.as_object() {
            for (k, v) in o {
                if k != "primary" {
                    self.loss(
                        &path("/fonts", k),
                        "unused-v1-font",
                        "v1 renderer only reads fonts.primary",
                        v,
                    );
                }
            }
        }
        if let Some(primary) = fonts["primary"].as_object() {
            for (k, v) in primary {
                match k.as_str() {
                    "family" => {
                        if let Some(s) = v.as_str() {
                            self.out["tokens"]["fonts"]["ui"] = json!({"family":css::family(s)});
                            self.loss(
                                "/fonts/primary/family",
                                "font-availability",
                                "family names retained; no fonts bundled or fetched",
                                v,
                            );
                            for slot in [
                                "window.interactive",
                                "window.clickThrough",
                                "button",
                                "input",
                                "select",
                                "headerTitle",
                                "taskText",
                                "progressText",
                            ] {
                                self.slot(
                                    slot,
                                    None,
                                    &json!({"text":{"family":"@fonts.ui.family"}}),
                                );
                            }
                        } else {
                            self.loss(
                                "/fonts/primary/family",
                                "unsupported-font",
                                "expected family list string",
                                v,
                            );
                        }
                    }
                    "sizes" | "weights" => {
                        if let Some(o) = v.as_object() {
                            for (name, v) in o {
                                let n = v.as_f64().or_else(|| v.as_str().and_then(css::length));
                                if let Some(n) = n.filter(|n| {
                                    if k == "weights" {
                                        (1.0..=1000.).contains(n)
                                    } else {
                                        *n >= 0.
                                    }
                                }) {
                                    if !self.out["tokens"]["fonts"][k].is_object() {
                                        self.out["tokens"]["fonts"][k] = json!({});
                                    }
                                    self.out["tokens"]["fonts"][k][name] = json!(n);
                                } else {
                                    self.loss(
                                        &path(&path("/fonts/primary", k), name),
                                        "unsupported-font",
                                        "unsupported font size/weight",
                                        v,
                                    );
                                }
                            }
                        } else {
                            self.loss(
                                &path("/fonts/primary", k),
                                "unsupported-font",
                                "expected size/weight map",
                                v,
                            );
                        }
                    }
                    _ => self.loss(
                        &path("/fonts/primary", k),
                        "unsupported-font",
                        "unmapped font field",
                        v,
                    ),
                }
            }
        }
        for (name, slots) in [
            ("header", vec!["headerTitle"]),
            ("task", vec!["taskText"]),
            ("progress", vec!["progressText"]),
            ("ui", vec!["button", "select", "input"]),
        ] {
            if self.out["tokens"]["fonts"]["sizes"].get(name).is_some() {
                for slot in slots {
                    self.slot(
                        slot,
                        None,
                        &json!({"text":{"size":format!("@fonts.sizes.{name}")}}),
                    );
                }
            }
        }
        for (name, slot) in [("normal", "taskText"), ("bold", "progressText")] {
            if self.out["tokens"]["fonts"]["weights"].get(name).is_some() {
                self.slot(
                    slot,
                    None,
                    &json!({"text":{"weight":format!("@fonts.weights.{name}")}}),
                );
            }
        }
    }
    fn effects(&mut self, effects: &Value) {
        if let Some(o) = effects.as_object() {
            for (k, v) in o {
                let Some(s) = v.as_str() else {
                    self.loss(
                        &path("/effects", k),
                        "unsupported-effect",
                        "effect is not a CSS value string",
                        v,
                    );
                    continue;
                };
                if k == "borderRadius"
                    && let Some(n) = css::edges(s, false, "radius")
                {
                    self.out["tokens"]["radius"] = json!({"default":n});
                    for slot in BUTTON_EFFECT_SLOTS.iter().copied().chain([
                        "input",
                        "textarea",
                        "select",
                        "progressBar",
                    ]) {
                        self.slot(slot, None, &json!({"radius":"@radius.default"}));
                    }
                    continue;
                }
                let target = match k.as_str() {
                    "boxShadow" => Some("window"),
                    "buttonShadow" => Some("button"),
                    "textShadow" => Some("text"),
                    "glow" => Some("glow"),
                    "bevelInset" => Some("bevelInset"),
                    "bevelOutset" => Some("bevelOutset"),
                    _ => None,
                };
                if let Some(target) = target
                    && let Some(shadows) = css::shadows(s, &self.out["tokens"])
                {
                    self.out["tokens"]["shadows"][target] = shadows;
                    match k.as_str() {
                        "textShadow" => {
                            for slot in BUTTON_EFFECT_SLOTS
                                .iter()
                                .copied()
                                .chain(["taskText", "progressText"])
                            {
                                self.slot(
                                    slot,
                                    None,
                                    &json!({"text":{"shadow":format!("@shadows.{target}")}}),
                                );
                            }
                        }
                        "buttonShadow" => {
                            for slot in BUTTON_EFFECT_SLOTS {
                                self.slot(
                                    slot,
                                    None,
                                    &json!({"shadow":format!("@shadows.{target}")}),
                                );
                            }
                        }
                        "bevelInset" => {
                            for slot in ["input", "textarea", "select"] {
                                self.slot(
                                    slot,
                                    None,
                                    &json!({"shadow":format!("@shadows.{target}")}),
                                );
                            }
                        }
                        "boxShadow" => self.slot(
                            "progressBar",
                            None,
                            &json!({"shadow":format!("@shadows.{target}")}),
                        ),
                        _ => {}
                    }
                    let utility = match k.as_str() {
                        "textShadow" => Some(".text-shadow"),
                        "bevelInset" => Some(".bevel-inset"),
                        "bevelOutset" => Some(".bevel-outset"),
                        "glow" => Some(".glow"),
                        _ => None,
                    };
                    if let Some(selector) = utility {
                        self.loss(
                            &path("/effects", k),
                            "effect-utility-selector",
                            "effect token and known control usages retained; arbitrary v1 utility-class targets have no fixed v2 slot and require explicit token references",
                            &json!({"selector":selector,"effect":v}),
                        );
                    }
                    continue;
                }
                self.loss(&path("/effects",k),"unsupported-effect","inset/filter/blur or unsupported effect requires manual authoring; no system blur guessed",v);
            }
        }
    }
}

pub fn convert_v1(input: ConversionInput<'_>) -> Result<Conversion, Vec<Diagnostic>> {
    convert_depth(input, 0)
}
fn convert_depth(input: ConversionInput<'_>, depth: usize) -> Result<Conversion, Vec<Diagnostic>> {
    let source = input.document;
    let mut errors = vec![];
    if !super::resolve::bounded_input(source, &mut errors) {
        return Err(errors);
    }
    if detect_format(source) != Ok(ThemeFormat::V1) || depth > 16 {
        issue(
            &mut errors,
            "/format",
            "conversion-input",
            "converter requires v1 and at most 16 variant levels",
            source,
            Severity::Error,
        );
        return Err(errors);
    }
    let validation = super::super::validate_theme(source);
    for message in validation.errors {
        issue(
            &mut errors,
            "",
            "invalid-v1",
            &message,
            source,
            Severity::Error,
        );
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut c = Converter {
        out: json!({"format":2,"id":source["id"],"name":source["name"],"tokens":{"colors":{},"opacity":{"appliesTo":[]},"fonts":{},"shadows":{}},"slots":{},"images":{}}),
        diagnostics: vec![],
    };
    for message in validation.warnings {
        c.loss("", "v1-warning", &message, source);
    }
    for (k, v) in source.as_object().expect("validated") {
        match k.as_str() {
            "id" | "name" | "format" | "colors" | "fonts" | "effects" | "styles" | "components"
            | "customCSS" | "cssFile" | "variants" => {}
            "version" | "author" | "description" | "supportsTransparency" | "sounds" => {
                c.out[k] = v.clone();
            }
            _ => c.loss(
                &path("", k),
                "unmapped-field",
                "v1 field is not interpreted by the v2 converter",
                v,
            ),
        }
    }
    if let Some(v) = source.get("colors")
        && let Some(colors) = c.colors(v, "/colors", "")
    {
        c.out["tokens"]["colors"] = colors;
    }
    for (key, slot, state) in [
        ("primary", "taskText", None),
        ("muted", "taskText", Some("completed")),
        ("secondary", "progressText", None),
    ] {
        if c.out["tokens"]["colors"]["text"].get(key).is_some() {
            c.slot(
                slot,
                state,
                &json!({"text":{"color":format!("@text.{key}")}}),
            );
        }
    }
    c.fonts(&source["fonts"]);
    c.effects(&source["effects"]);
    // Filename sugar is explicit in output; unknown assets are never silently claimed converted.
    let mut files = input.asset_files.to_vec();
    files.sort();
    files.dedup();
    for asset in &files {
        if !safe_asset(asset) {
            c.loss(
                "/assets",
                "unsafe-asset",
                "unsafe asset path was not mapped",
                &json!(asset),
            );
            continue;
        }
        let stem = asset.rsplit_once('.').map(|(s, _)| s).unwrap_or(asset);
        let mapping = match stem {
            "main_bg" => Some(("window.interactive", None, "stretch")),
            "titlebar_bg" => Some(("header", None, "cover")),
            "button_normal" => Some(("button", None, "cover")),
            "button_hover" => Some(("button", Some("hover"), "cover")),
            "button_pressed" => Some(("button", Some("active"), "cover")),
            "progress_bg" => Some(("progressBar", None, "cover")),
            "progress_fill" => Some(("progressFill", None, "cover")),
            _ => None,
        };
        if let Some((slot, state, mode)) = mapping {
            let image = json!({"asset":asset,"mode":mode});
            c.slot(slot, state, &json!({"image":image}));
            if state.is_none() {
                c.out["images"][slot] = image.clone();
            }
            if stem == "main_bg" {
                c.slot("window.clickThrough", None, &json!({"image":image}));
                c.out["images"]["window.clickThrough"] = image;
            }
        } else if ![".wav", ".ogg", ".mp3", ".m4a"]
            .iter()
            .any(|s| asset.to_lowercase().ends_with(s))
        {
            c.loss(
                "/assets",
                "unmapped-asset",
                "no known filename-to-slot convention; asset not copied by pure converter",
                &json!(asset),
            );
        }
    }
    if let Some(modes) = source["styles"].as_object() {
        for (mode, v) in modes {
            if let Some(styles) = v.as_object() {
                for (component, props) in styles {
                    let p = path(&path("/styles", mode), component);
                    let slot = match (mode.as_str(), component.as_str()) {
                        ("interactive", "container") => Some("window.interactive"),
                        ("clickThrough", "container") => Some("window.clickThrough"),
                        ("interactive", "header") => Some("header"),
                        ("interactive", "templateSection") => Some("templateSection"),
                        ("interactive", "tasksSection") => Some("tasksSection"),
                        _ => None,
                    };
                    if let Some(slot) = slot {
                        let style = c.declarations(props, &p, slot, None);
                        c.slot(slot, None, &style);
                    } else {
                        c.loss(&p,"unsupported-mode-slot","no mode-qualified section slot in v2; manual application decision required",props);
                    }
                }
            } else {
                c.loss(
                    &path("/styles", mode),
                    "unsupported-style",
                    "expected mode map",
                    v,
                );
            }
        }
    }
    if let Some(components) = source["components"].as_object() {
        for (name, props) in components {
            let slot = match name.as_str() {
                "taskItem" => "taskRow",
                "button" | "input" | "select" | "progressBar" | "progressFill" | "checkbox" => name,
                _ => {
                    c.loss(
                        &path("/components", name),
                        "unsupported-component",
                        "v1 component is not recognized",
                        props,
                    );
                    continue;
                }
            };
            let p = path("/components", name);
            // themeCssComponents.ts uses isRecord, not presence/truthiness.
            if let Some(states) = props.get("states").and_then(Value::as_object) {
                if let Some(o) = props.as_object() {
                    for (k, v) in o {
                        if k != "states" {
                            c.loss(
                                &path(&p, k),
                                "v1-ignored-base",
                                "v1 states branch ignores sibling base declarations",
                                v,
                            );
                        }
                    }
                }
                let mut translated = Vec::new();
                for (state, props) in states {
                    if matches!(state.as_str(), "hover" | "active" | "focus" | "disabled") {
                        let style = c.declarations(
                            props,
                            &path(&path(&p, "states"), state),
                            slot,
                            Some(state),
                        );
                        c.slot(slot, Some(state), &style);
                        translated.push((state.as_str(), style));
                    } else {
                        c.loss(
                            &path(&path(&p, "states"), state),
                            "unsupported-state",
                            "not a v1 component state",
                            props,
                        );
                    }
                }
                c.state_order(&translated, &path(&p, "states"));
            } else {
                let mut base = props.clone();
                if let Some(o) = base.as_object_mut()
                    && let Some(states) = o.shift_remove("states")
                {
                    c.loss(
                        &path(&p, "states"),
                        "unsupported-state",
                        "non-record states is not a v1 state branch; sibling base declarations retained",
                        &states,
                    );
                }
                let mut style = c.declarations(&base, &p, slot, None);
                c.component_font_size(slot, &base, &p, &mut style);
                c.slot(slot, None, &style);
            }
        }
    }
    if let Some(css) = source.get("customCSS") {
        if let Some(s) = css.as_str() {
            c.css(s, "/customCSS");
        } else if let Some(o) = css.as_object() {
            for (k, v) in o {
                if let Some(s) = v.as_str() {
                    c.css(s, &path("/customCSS", k));
                } else {
                    c.loss(
                        &path("/customCSS", k),
                        "unsupported-selector",
                        "v1 .theme-* property maps have no fixed slot",
                        v,
                    );
                }
            }
        }
    }
    if source.get("cssFile").is_some() {
        if let Some(css) = input.css_file {
            c.css(css, "/cssFile");
        } else {
            c.loss(
                "/cssFile",
                "missing-css",
                "declared CSS text was not supplied; caller must load safely",
                &source["cssFile"],
            );
        }
    }
    // Expanded v1 variants preserve v1 defu semantics before translating the effective data.
    if let Some(variants) = source["variants"].as_object() {
        for (id, config) in variants {
            if let Some(config) = config.as_object() {
                for (key, value) in config {
                    if !matches!(
                        key.as_str(),
                        "name"
                            | "description"
                            | "colors"
                            | "effects"
                            | "fonts"
                            | "styles"
                            | "components"
                            | "backgrounds"
                            | "layout"
                            | "animations"
                            | "customCSS"
                            | "sounds"
                    ) {
                        c.loss(
                            &path(&path("/variants", id), key),
                            "unmapped-variant-field",
                            "v1 variant expansion ignores this field",
                            value,
                        );
                    }
                }
                let effective = super::super::create_variant_theme(
                    source.as_object().expect("validated"),
                    id,
                    config,
                );
                let effective = Value::Object(effective);
                let converted = convert_depth(
                    ConversionInput {
                        document: &effective,
                        ..input
                    },
                    depth + 1,
                )?;
                for mut d in converted.diagnostics {
                    d.location = format!("{}{}", path("/variants", id), d.location);
                    c.diagnostics.push(d);
                }
                let mut base = c.out.clone();
                base.as_object_mut()
                    .expect("initialized")
                    .shift_remove("variants");
                let mut patch = variant_delta(
                    &converted.theme,
                    &base,
                    &path("/variants", id),
                    &mut c.diagnostics,
                )
                .unwrap_or_else(|| json!({}));
                if let Some(o) = patch.as_object_mut() {
                    o.shift_remove("format");
                    o.shift_remove("id");
                    o.shift_remove("variants");
                }
                if !c.out["variants"].is_object() {
                    c.out["variants"] = json!({});
                }
                c.out["variants"][id] = patch;
            } else {
                c.loss(
                    &path("/variants", id),
                    "unsupported-variant",
                    "invalid v1 variant ignored",
                    config,
                );
            }
        }
    }
    match Theme::parse(&c.out) {
        Ok(t) => c.diagnostics.extend(t.diagnostics().iter().cloned()),
        Err(mut d) => {
            for item in &mut d {
                item.message = format!("converted output failed validation: {}", item.message);
            }
            return Err(d);
        }
    }
    sort(&mut c.diagnostics);
    Ok(Conversion {
        theme: c.out,
        diagnostics: c.diagnostics,
        provisional: source.get("cssFile").is_some(),
    })
}
fn variant_delta(new: &Value, base: &Value, p: &str, d: &mut Vec<Diagnostic>) -> Option<Value> {
    if new == base {
        return None;
    }
    if let (Some(new), Some(base)) = (new.as_object(), base.as_object()) {
        let mut result = serde_json::Map::new();
        for (k, v) in new {
            if let Some(value) =
                variant_delta(v, base.get(k).unwrap_or(&Value::Null), &path(p, k), d)
            {
                result.insert(k.clone(), value);
            }
        }
        for (k, inherited) in base {
            if new.contains_key(k) {
                continue;
            }
            if k == "opacityAffects" && inherited == &json!(true) {
                // Absence cannot undo an inherited defu boolean. An effective
                // variant with an opaque fill must explicitly clear this flag.
                result.insert(k.clone(), json!(false));
            } else {
                issue(
                    d,
                    &path(p, k),
                    "variant-inherited-value",
                    "effective v1 conversion omits this value but v2 defu cannot delete inherited keys; inherited value retained; author a separate slot/token",
                    inherited,
                    Severity::Warning,
                );
            }
        }
        return Some(Value::Object(result));
    }
    if let (Some(new), Some(base)) = (new.as_array(), base.as_array()) {
        if new.ends_with(base) {
            return Some(json!(new[..new.len() - base.len()]));
        }
        issue(
            d,
            p,
            "variant-array-replacement",
            "v2 defu cannot replace inherited arrays; retained base array; author a new token and reference it",
            &json!({"requested":new,"inherited":base}),
            Severity::Warning,
        );
        return None;
    }
    Some(new.clone())
}
