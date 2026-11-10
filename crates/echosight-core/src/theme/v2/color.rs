//! Bounded sRGB parser: hex, comma rgb/rgba, and the sixteen basic CSS colors.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}
impl Rgba {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase();
        let named = match s.as_str() {
            "transparent" => Some("#00000000"),
            "black" => Some("#000000"),
            "silver" => Some("#c0c0c0"),
            "gray" | "grey" => Some("#808080"),
            "white" => Some("#ffffff"),
            "maroon" => Some("#800000"),
            "red" => Some("#ff0000"),
            "purple" => Some("#800080"),
            "fuchsia" => Some("#ff00ff"),
            "green" => Some("#008000"),
            "lime" => Some("#00ff00"),
            "olive" => Some("#808000"),
            "yellow" => Some("#ffff00"),
            "navy" => Some("#000080"),
            "blue" => Some("#0000ff"),
            "teal" => Some("#008080"),
            "aqua" => Some("#00ffff"),
            "pink" => Some("#ffc0cb"),
            _ => None,
        };
        let text = named.unwrap_or(&s);
        if let Some(hex) = text.strip_prefix('#') {
            if !matches!(hex.len(), 3 | 4 | 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let hex = if hex.len() <= 4 {
                hex.chars().flat_map(|c| [c, c]).collect::<String>()
            } else {
                hex.into()
            };
            let byte = |i| {
                u8::from_str_radix(&hex[i..i + 2], 16)
                    .ok()
                    .map(|x| x as f64 / 255.)
            };
            return Some(Self {
                r: byte(0)?,
                g: byte(2)?,
                b: byte(4)?,
                a: if hex.len() == 8 { byte(6)? } else { 1. },
            });
        }
        let (body, alpha) = if let Some(x) = text.strip_prefix("rgba(") {
            (x, true)
        } else {
            (text.strip_prefix("rgb(")?, false)
        };
        let parts: Vec<_> = body.strip_suffix(')')?.split(',').map(str::trim).collect();
        if parts.len() != if alpha { 4 } else { 3 } {
            return None;
        }
        let channel = |i: usize| -> Option<f64> {
            let n = parts[i].parse::<f64>().ok()?;
            (n.is_finite() && (0.0..=255.).contains(&n)).then_some(n / 255.)
        };
        let a = if alpha {
            parts[3].parse::<f64>().ok()?
        } else {
            1.
        };
        if !a.is_finite() || !(0.0..=1.).contains(&a) {
            return None;
        }
        Some(Self {
            r: channel(0)?,
            g: channel(1)?,
            b: channel(2)?,
            a,
        })
    }
}
