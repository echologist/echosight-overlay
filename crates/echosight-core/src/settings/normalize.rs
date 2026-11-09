use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::jscompat::{js_number, js_round, js_trim};

use super::{
    CURRENT_SETTINGS_VERSION, HotkeyAction, HotkeySettings, Settings, primary_modifier_label,
};

/// Renderer defaults depend on navigator.platform, explicitly supplied by the shell.
pub fn renderer_defaults(platform: &str) -> Settings {
    Settings::with_hotkeys(HotkeySettings::with_primary_modifier(
        primary_modifier_label(platform),
    ))
}

/// `None` means JavaScript undefined (fallback); explicit JSON null coerces to 0.
pub fn normalize_percentage(value: Option<&Value>, fallback: u8) -> u8 {
    let numeric = value.map(js_number).unwrap_or(f64::NAN);
    if numeric.is_finite() {
        js_round(numeric).clamp(0.0, 100.0) as u8
    } else {
        fallback
    }
}

fn normalized_string(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .map(js_trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback)
        .into()
}

/// Normalize raw data before typed decoding. Unknown fields (including legacy
/// backgroundColor) are dropped; hotkey spellings/order are preserved here.
/// Accelerator conversion is a separate main-process rule.
pub fn normalize_settings_value(input: &Value, fallback: &Settings) -> Value {
    let field = |name: &str| input.as_object().and_then(|object| object.get(name));
    let legacy = field("backgroundColor")
        .and_then(Value::as_str)
        .map(js_trim)
        .filter(|s| !s.is_empty());
    let hotkeys = field("hotkeys").and_then(Value::as_object);
    let sounds = field("sounds").and_then(Value::as_object);
    let hotkeys: serde_json::Map<String, Value> = HotkeyAction::ALL
        .into_iter()
        .map(|action| {
            (
                action.as_str().into(),
                Value::String(normalized_string(
                    hotkeys.and_then(|h| h.get(action.as_str())),
                    fallback.hotkeys.get(action),
                )),
            )
        })
        .collect();
    json!({
        "settingsVersion": CURRENT_SETTINGS_VERSION,
        "transparency": normalize_percentage(field("transparency"), fallback.transparency),
        "theme": normalized_string(field("theme"), legacy.unwrap_or(&fallback.theme)),
        "hotkeys": hotkeys,
        "sounds": {
            "enabled": sounds.and_then(|s| s.get("enabled")).and_then(Value::as_bool).unwrap_or(fallback.sounds.enabled),
            "volume": normalize_percentage(sounds.and_then(|s| s.get("volume")), fallback.sounds.volume)
        }
    })
}

pub fn normalize_settings(input: &Value, fallback: &Settings) -> Settings {
    serde_json::from_value(normalize_settings_value(input, fallback))
        .expect("normalization constructs a complete typed settings value")
}

/// Migration is field-wise JS strict equality, NOT whole-document equality.
/// Ignored extra fields alone do not cause a save; legacy backgroundColor does.
pub fn should_persist_settings_migration(input: &Value, settings: &Settings) -> bool {
    let Some(source) = input.as_object() else {
        return true;
    };
    if source.get("settingsVersion").and_then(Value::as_f64)
        != Some(f64::from(CURRENT_SETTINGS_VERSION))
        || source.contains_key("backgroundColor")
    {
        return true;
    }
    let hotkeys_equal = source
        .get("hotkeys")
        .and_then(Value::as_object)
        .is_some_and(|hotkeys| {
            HotkeyAction::ALL.into_iter().all(|action| {
                hotkeys.get(action.as_str()).and_then(Value::as_str)
                    == Some(settings.hotkeys.get(action))
            })
        });
    let sounds_equal = source
        .get("sounds")
        .and_then(Value::as_object)
        .is_some_and(|sounds| {
            sounds.get("enabled").and_then(Value::as_bool) == Some(settings.sounds.enabled)
                && sounds.get("volume").and_then(Value::as_f64)
                    == Some(f64::from(settings.sounds.volume))
        });
    source.get("transparency").and_then(Value::as_f64) != Some(f64::from(settings.transparency))
        || source.get("theme").and_then(Value::as_str) != Some(settings.theme.as_str())
        || !hotkeys_equal
        || !sounds_equal
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsMigration {
    pub settings: Settings,
    /// Persist the normalized document only when true.
    pub changed: bool,
}

pub fn migrate_settings(input: &Value, fallback: &Settings) -> SettingsMigration {
    let settings = normalize_settings(input, fallback);
    let changed = should_persist_settings_migration(input, &settings);
    SettingsMigration { settings, changed }
}
