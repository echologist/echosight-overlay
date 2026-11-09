//! Renderer-facing accessors retain authored omissions: no hidden layout defaults.
use super::Rgba;
use serde::Deserialize;
use serde_json::Value;
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStyle {
    properties: Value,
}
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub color: Rgba,
    pub position: f64,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid(Rgba),
    Linear {
        angle: f64,
        stops: Vec<GradientStop>,
    },
    Radial {
        center: [f64; 2],
        radius: f64,
        stops: Vec<GradientStop>,
    },
}
impl Fill {
    pub fn solid(&self) -> Option<Rgba> {
        if let Self::Solid(c) = self {
            Some(*c)
        } else {
            None
        }
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub color: Option<String>,
    pub size: Option<f64>,
    pub weight: Option<f64>,
    pub family: Option<Vec<String>>,
    pub transform: Option<String>,
    pub letter_spacing: Option<f64>,
    pub strikethrough: Option<bool>,
    pub shadow: Option<Vec<Shadow>>,
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Shadow {
    pub x: f64,
    pub y: f64,
    pub blur: Option<f64>,
    pub spread: Option<f64>,
    pub color: String,
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Border {
    pub color: Option<String>,
    pub width: Option<f64>,
    pub style: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bevel {
    pub light: String,
    pub dark: String,
    pub inner_light: Option<String>,
    pub inner_dark: Option<String>,
    pub width: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ImageStyle {
    pub asset: String,
    pub mode: String,
    pub insets: Option<[f64; 4]>,
}
impl ResolvedStyle {
    pub(super) fn new(properties: Value) -> Self {
        Self { properties }
    }
    pub fn properties(&self) -> &Value {
        &self.properties
    }
    pub fn number(&self, key: &str) -> Option<f64> {
        self.properties[key].as_f64()
    }
    pub fn text(&self) -> Option<TextStyle> {
        serde_json::from_value(self.properties["text"].clone()).ok()
    }
    pub fn border(&self) -> Option<Border> {
        serde_json::from_value(self.properties["border"].clone()).ok()
    }
    pub fn bevel(&self) -> Option<Bevel> {
        serde_json::from_value(self.properties["bevel"].clone()).ok()
    }
    pub fn image(&self) -> Option<ImageStyle> {
        serde_json::from_value(self.properties["image"].clone()).ok()
    }
    pub fn shadows(&self) -> Option<Vec<Shadow>> {
        serde_json::from_value(self.properties["shadow"].clone()).ok()
    }
    /// CSS order: top, right, bottom, left. Radius arrays use TL, TR, BR, BL.
    pub fn edges(&self, key: &str) -> Option<[f64; 4]> {
        let v = &self.properties[key];
        if let Some(n) = v.as_f64() {
            return Some([n; 4]);
        }
        let a = v.as_array()?;
        match a.len() {
            2 => Some([
                a[0].as_f64()?,
                a[1].as_f64()?,
                a[0].as_f64()?,
                a[1].as_f64()?,
            ]),
            4 => Some([
                a[0].as_f64()?,
                a[1].as_f64()?,
                a[2].as_f64()?,
                a[3].as_f64()?,
            ]),
            _ => None,
        }
    }
    pub fn fill(&self) -> Option<Fill> {
        let v = &self.properties["fill"];
        if let Some(s) = v.as_str() {
            return Rgba::parse(s).map(Fill::Solid);
        }
        let alpha = v["alpha"].as_f64().unwrap_or(1.);
        let color = |v: &Value| -> Option<Rgba> {
            let mut c = Rgba::parse(v.as_str()?)?;
            c.a *= alpha;
            Some(c)
        };
        if v.get("color").is_some() {
            return color(&v["color"]).map(Fill::Solid);
        }
        let stops = |g: &Value| -> Option<Vec<GradientStop>> {
            g["stops"]
                .as_array()?
                .iter()
                .map(|s| {
                    Some(GradientStop {
                        color: color(&s[0])?,
                        position: s[1].as_f64()?,
                    })
                })
                .collect()
        };
        if let Some(g) = v.get("gradient") {
            return Some(Fill::Linear {
                angle: g["angle"].as_f64()?,
                stops: stops(g)?,
            });
        }
        let g = v.get("radial")?;
        Some(Fill::Radial {
            center: [g["center"][0].as_f64()?, g["center"][1].as_f64()?],
            radius: g["radius"].as_f64()?,
            stops: stops(g)?,
        })
    }
}
