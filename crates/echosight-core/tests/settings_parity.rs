//! Expectations are captured by importing the unchanged TypeScript implementation.
use echosight_core::settings::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("golden/settings/parity.json")).unwrap()
}

fn actual(section: &str, case: &Value) -> Value {
    let platform = case["platform"].as_str().unwrap_or("Win32");
    match section {
        "bootstrap" => json!(Settings::default()),
        "defaults" => {
            assert_eq!(json!(primary_modifier_label(platform)), case["primary"]);
            json!(renderer_defaults(platform))
        }
        "settings" => {
            let fallback = case
                .get("fallback")
                .map(|v| serde_json::from_value(v.clone()).unwrap())
                .unwrap_or_else(|| renderer_defaults(platform));
            let migration = migrate_settings(&case["input"], &fallback);
            assert_eq!(
                json!(migration.changed),
                case["changed"],
                "migration decision: {case}"
            );
            assert_eq!(
                normalize_settings_value(&case["input"], &fallback),
                json!(migration.settings)
            );
            assert_eq!(
                json!(echosight_core::storage::json_format(&json!(migration.settings)).unwrap()),
                case["expectedJson"]
            );
            assert!(!should_persist_settings_migration(
                &json!(migration.settings),
                &migration.settings
            ));
            json!(migration.settings)
        }
        "percentages" => json!(normalize_percentage(
            case.get("input"),
            case["fallback"].as_u64().unwrap() as u8
        )),
        "conversions" => json!(convert_hotkey_format(
            case.get("input"),
            case["fallback"]
                .as_str()
                .unwrap_or("CommandOrControl+Shift+T")
        )),
        "hotkeySets" => json!(normalize_accelerator_hotkeys(
            &case["input"],
            &HotkeySettings::default()
        )),
        "displays" => json!(format_hotkey_for_display(
            case["input"].as_str().unwrap(),
            platform
        )),
        "keys" => json!(hotkey_recording_result(
            &serde_json::from_value(case["event"].clone()).unwrap(),
            platform
        )),
        "recorders" => recorder_trace(case),
        _ => panic!("unknown fixture section {section}"),
    }
}

#[derive(Default)]
struct Shell {
    controls: serde_json::Map<String, Value>,
    recorded: Vec<Value>,
    alerts: Vec<String>,
    recording_changes: Vec<bool>,
    consumed: bool,
    now: u64,
    timers: Vec<u64>,
}

impl Shell {
    fn apply(&mut self, effects: Vec<RecordingEffect>) {
        for effect in effects {
            match effect {
                RecordingEffect::Begin { action } => {
                    self.controls[action.as_str()] = json!({ "value": "Press keys now...", "recording": true, "button": "Cancel" });
                }
                RecordingEffect::End { action, restore } => {
                    let control = self.controls.get_mut(action.as_str()).unwrap();
                    if let Some(value) = restore {
                        control["value"] = json!(value);
                    }
                    control["recording"] = json!(false);
                    control["button"] = json!("Record");
                }
                RecordingEffect::SuspendGlobalHotkeys => self.recording_changes.push(true),
                RecordingEffect::ReregisterGlobalHotkeys => self.recording_changes.push(false),
                RecordingEffect::ConsumeKeydown => self.consumed = true,
                RecordingEffect::Alert(message) => self.alerts.push(message),
                RecordingEffect::Recorded { action, hotkey } => {
                    self.controls.get_mut(action.as_str()).unwrap()["value"] = json!(hotkey);
                    self.recorded
                        .push(json!({ "action": action, "hotkey": hotkey }));
                }
                RecordingEffect::ScheduleStop { delay_ms } => self.timers.push(self.now + delay_ms),
            }
        }
    }
}

fn recorder_trace(case: &Value) -> Value {
    let platform = case["platform"].as_str().unwrap();
    let mut recorder = HotkeyRecorder::default();
    let mut shell = Shell::default();
    for action in HotkeyAction::ALL {
        if case["missing"].as_str() == Some(action.as_str()) {
            continue;
        }
        shell.controls.insert(action.as_str().into(), json!({
            "value": format_hotkey_for_display(case["hotkeys"][action.as_str()].as_str().unwrap(), platform),
            "recording": false, "button": "Record"
        }));
    }
    let mut snapshots = Vec::new();
    for op in case["ops"].as_array().unwrap() {
        shell.consumed = false;
        if let Some(action) = op["record"].as_str() {
            let previous = shell.controls.get(action).and_then(|v| v["value"].as_str());
            let effects = recorder.record(action, previous);
            shell.apply(effects);
        }
        if op.get("stop").is_some() {
            shell.apply(recorder.stop());
        }
        if let Some(ms) = op["advance"].as_u64() {
            shell.now += ms;
            while let Some(index) = shell.timers.iter().position(|time| *time <= shell.now) {
                shell.timers.remove(index);
                shell.apply(recorder.finish_deferred_stop());
            }
        }
        if let Some(event) = op.get("key") {
            shell.apply(recorder.handle_keydown(
                &serde_json::from_value(event.clone()).unwrap(),
                if case["noGetter"] == true {
                    None
                } else {
                    Some(&case["hotkeys"])
                },
                platform,
            ));
        }
        snapshots.push(json!({ "active": recorder.is_recording(), "consumed": shell.consumed,
            "recorded": shell.recorded, "alerts": shell.alerts, "recordingChanges": shell.recording_changes,
            "controls": shell.controls }));
    }
    json!(snapshots)
}

macro_rules! parity_test {
    ($name:ident, $section:literal) => {
        #[test]
        fn $name() {
            let data = fixture();
            for (index, case) in data[$section].as_array().unwrap().iter().enumerate() {
                assert_eq!(
                    actual($section, case),
                    case["expected"],
                    "{} case {index}: {case}",
                    $section
                );
            }
        }
    };
}

parity_test!(settings_normalization_matches_typescript, "settings");
parity_test!(percentage_coercions_match_typescript, "percentages");
parity_test!(accelerator_conversion_matches_typescript, "conversions");
parity_test!(hotkey_sets_match_typescript, "hotkeySets");
parity_test!(platform_display_matches_typescript, "displays");
parity_test!(key_recording_results_match_typescript, "keys");
parity_test!(renderer_defaults_match_typescript, "defaults");

#[test]
fn bootstrap_defaults_match_typescript() {
    assert_eq!(
        actual("bootstrap", &Value::Null),
        fixture()["bootstrapDefaults"]
    );
}

#[test]
fn recorder_traces_match_typescript() {
    for case in fixture()["recorders"].as_array().unwrap() {
        assert_eq!(
            actual("recorders", case),
            case["snapshots"],
            "{}",
            case["name"]
        );
    }
}
