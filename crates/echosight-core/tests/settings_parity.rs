//! Expectations are captured by importing the unchanged TypeScript implementation.
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("golden/settings/parity.json")).unwrap()
}

// RED checkpoint: replace this adapter with production APIs, never fixture-derived results.
fn actual(_section: &str, _case: &Value) -> Value {
    Value::Null
}

macro_rules! parity_test {
    ($name:ident, $section:literal) => {
        #[test]
        fn $name() {
            let data = fixture();
            for (index, case) in data[$section].as_array().unwrap().iter().enumerate() {
                assert_eq!(actual($section, case), case["expected"], "{} case {index}: {case}", $section);
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
    assert_eq!(actual("bootstrap", &Value::Null), fixture()["bootstrapDefaults"]);
}

#[test]
fn recorder_traces_match_typescript() {
    for case in fixture()["recorders"].as_array().unwrap() {
        assert_eq!(actual("recorders", case), case["snapshots"], "{}", case["name"]);
    }
}
