//! JSON expectations are executed TS output, never handwritten Rust oracles.
use echosight_core::theme::{
    compare_theme_version, create_variant_theme, deep_merge_defu, get_asset_type, get_mime_type,
    get_supported_theme_asset_extensions, is_audio_asset_extension, validate_theme,
    validate_theme_variant_config,
};
use serde_json::{Value, json};
use std::path::Path;

fn cases(name: &str) -> Vec<Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/themes")
        .join(format!("{name}.json"));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn raw_theme_validation_matches_typescript() {
    for case in cases("validation") {
        let input = case["input"].clone();
        let actual = serde_json::to_value(validate_theme(&input)).unwrap();
        assert_eq!(actual, case["expected"], "{}", case["name"]);
        assert_eq!(input, case["input"], "validator mutated input");
    }
}

#[test]
fn variant_validation_matches_typescript() {
    for case in cases("variants") {
        let input = &case["input"];
        let actual = validate_theme_variant_config(
            input["themeId"].as_str().unwrap(),
            input["variantId"].as_str().unwrap(),
            &input["config"],
        );
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            case["expected"],
            "{}",
            case["name"]
        );
    }
}

#[test]
fn variant_creation_matches_typescript_without_mutation() {
    for case in cases("merges") {
        let input = case["input"].clone();
        let actual = create_variant_theme(
            input["base"].as_object().unwrap(),
            input["variantId"].as_str().unwrap(),
            input["config"].as_object().unwrap(),
        );
        assert_eq!(json!(actual), case["expected"], "{}", case["name"]);
        assert_eq!(input, case["input"], "variant creation mutated input");
    }
}

#[test]
fn defu_merge_matches_installed_dependency() {
    for case in cases("defu") {
        let actual = deep_merge_defu(
            case["input"]["overrides"].as_object().unwrap(),
            case["input"]["defaults"].as_object().unwrap(),
        );
        assert_eq!(json!(actual), case["expected"], "{}", case["name"]);
    }
}

#[test]
fn asset_helpers_match_typescript() {
    for case in cases("helpers") {
        let input = &case["input"];
        let name = input["assetName"].as_str().unwrap();
        let ext = input["extension"].as_str().unwrap();
        let actual = json!({
            "assetType": get_asset_type(name, ext), "mimeType": get_mime_type(ext),
            "audio": is_audio_asset_extension(ext)
        });
        assert_eq!(actual, case["expected"], "{}", case["name"]);
    }
    assert_eq!(
        json!(get_supported_theme_asset_extensions()),
        cases("extensions")[0]["expected"]
    );
}

#[test]
fn version_comparison_matches_real_library_source() {
    for case in cases("versions") {
        let input = &case["input"];
        assert_eq!(
            compare_theme_version(&input["left"], &input["right"]),
            case["expected"].as_f64().unwrap(),
            "{}: {input}",
            case["name"]
        );
    }
}

#[test]
fn all_eleven_bundled_themes_validate_and_preserve_fields() {
    let fixtures = cases("bundled");
    assert_eq!(fixtures.len(), 11);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for case in fixtures {
        let path = case["name"].as_str().unwrap();
        let current: Value =
            serde_json::from_slice(&std::fs::read(root.join(path)).unwrap()).unwrap();
        assert_eq!(
            current, case["input"],
            "bundle changed; regenerate fixture: {path}"
        );
        let result = validate_theme(&current);
        assert!(result.errors.is_empty(), "{path}: {:?}", result.errors);
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            case["expected"],
            "{path}"
        );
    }
}
