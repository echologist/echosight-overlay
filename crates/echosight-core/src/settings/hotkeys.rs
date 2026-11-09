//! Source-compatible accelerator text handling, not OS registration or validation.
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::jscompat::{js_trim, utf16_len};

use super::{HotkeyAction, HotkeySettings};

pub fn primary_modifier_label(platform: &str) -> &'static str {
    let platform = platform.to_ascii_lowercase();
    if ["mac", "iphone", "ipad", "ipod"]
        .iter()
        .any(|name| platform.contains(name))
    {
        "Cmd"
    } else {
        "Ctrl"
    }
}

/// Exact renderer substitution: case-sensitive, no trimming, no Control alias,
/// no sorting/deduplication. Do not use accelerator conversion for UI display.
pub fn format_hotkey_for_display(hotkey: &str, platform: &str) -> String {
    static PATTERNS: OnceLock<[Regex; 4]> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        ["CommandOrControl", "Command", "Cmd", "Ctrl"].map(|word| {
            Regex::new(&format!(r"(?-u:\b{word}\b)")).expect("constant ASCII word regex")
        })
    });
    let primary = primary_modifier_label(platform);
    patterns
        .iter()
        .zip([primary, "Cmd", primary, primary])
        .fold(hotkey.to_owned(), |text, (pattern, replacement)| {
            pattern.replace_all(&text, replacement).into_owned()
        })
}

/// Main-process rule: aliases become CommandOrControl and repeated *exact*
/// primary tokens are removed. Existing CommandOrControl anywhere in the input
/// bypasses alias replacement, preserving the upstream mixed-alias quirk.
/// Empty '+' tokens are dropped; arbitrary keys are not validated.
pub fn convert_hotkey_format(input: Option<&Value>, fallback: &str) -> String {
    let Some(hotkey) = input
        .and_then(Value::as_str)
        .filter(|s| !js_trim(s).is_empty())
    else {
        return fallback.into();
    };
    static PATTERNS: OnceLock<[Regex; 4]> = OnceLock::new();
    let converted = if hotkey.contains("CommandOrControl") {
        hotkey.to_owned()
    } else {
        PATTERNS
            .get_or_init(|| {
                ["Ctrl", "Control", "Cmd", "Command"].map(|word| {
                    Regex::new(&format!(r"(?i-u:\b{word}\b)")).expect("constant ASCII alias regex")
                })
            })
            .iter()
            .fold(hotkey.to_owned(), |text, pattern| {
                pattern.replace_all(&text, "CommandOrControl").into_owned()
            })
    };
    let mut has_primary = false;
    converted
        .split('+')
        .map(js_trim)
        .filter(|part| !part.is_empty())
        .filter(|part| {
            if *part == "CommandOrControl" {
                if has_primary {
                    return false;
                }
                has_primary = true;
            }
            true
        })
        .collect::<Vec<_>>()
        .join("+")
}

pub fn normalize_accelerator_hotkeys(input: &Value, fallback: &HotkeySettings) -> HotkeySettings {
    let mut result = fallback.clone();
    for action in HotkeyAction::ALL {
        result.set(
            action,
            convert_hotkey_format(input.get(action.as_str()), fallback.get(action)),
        );
    }
    result
}

/// Same comparison as the recorder: display, JS trim, lowercase. Deliberately
/// does not reorder modifiers, strip interior spaces or convert Control.
pub fn hotkey_comparison_key(hotkey: &str, platform: &str) -> String {
    js_trim(&format_hotkey_for_display(hotkey, platform)).to_lowercase()
}

/// The raw variant preserves source insertion order and its first-match behavior:
/// an unknown first matching action masks a later known action. Normalized
/// Settings only contains the five known actions, in source order.
pub fn find_duplicate_hotkey_action(
    hotkeys: &Value,
    current_action: HotkeyAction,
    hotkey: &str,
    platform: &str,
) -> Option<HotkeyAction> {
    let normalized = hotkey_comparison_key(hotkey, platform);
    let entries = hotkeys.as_object()?;
    entries
        .iter()
        .find(|(action, assigned)| {
            action.as_str() != current_action.as_str()
                && assigned
                    .as_str()
                    .is_some_and(|assigned| hotkey_comparison_key(assigned, platform) == normalized)
        })
        .and_then(|(action, _)| HotkeyAction::parse(action))
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEvent {
    pub key: String,
    #[serde(default)]
    pub ctrl_key: bool,
    #[serde(default)]
    pub meta_key: bool,
    #[serde(default)]
    pub alt_key: bool,
    #[serde(default)]
    pub shift_key: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyRecordingResult {
    pub hotkey: Option<String>,
    pub warning: Option<String>,
}

/// Bare Escape is ignored, Shift+Escape warns, Ctrl/Alt+Escape is recordable.
/// Cancellation belongs to `HotkeyRecorder::stop`, as in the TS implementation.
pub fn hotkey_recording_result(event: &KeyEvent, platform: &str) -> Option<HotkeyRecordingResult> {
    let main = match event.key.as_str() {
        "Control" | "Alt" | "Shift" | "Meta" => return None,
        " " => "Space".into(),
        "ArrowUp" => "Up".into(),
        "ArrowDown" => "Down".into(),
        "ArrowLeft" => "Left".into(),
        "ArrowRight" => "Right".into(),
        key if utf16_len(key) == 1 => key.to_uppercase(),
        key => key.into(),
    };
    if main.is_empty() {
        return None;
    }
    let primary = primary_modifier_label(platform);
    let mut keys = Vec::new();
    if event.ctrl_key || event.meta_key {
        keys.push(primary);
    }
    if event.alt_key {
        keys.push("Alt");
    }
    if event.shift_key {
        keys.push("Shift");
    }
    keys.push(&main);
    let function = matches!(
        main.as_str(),
        "F1" | "F2" | "F3" | "F4" | "F5" | "F6" | "F7" | "F8" | "F9" | "F10" | "F11" | "F12"
    );
    if event.ctrl_key || event.meta_key || event.alt_key || function {
        Some(HotkeyRecordingResult {
            hotkey: Some(keys.join("+")),
            warning: None,
        })
    } else if keys.len() >= 2 {
        Some(HotkeyRecordingResult {
            hotkey: None,
            warning: Some(format!(
                "Please use {primary}+{main}, Alt+{main}, or a function key to avoid conflicts with normal typing."
            )),
        })
    } else {
        None
    }
}
