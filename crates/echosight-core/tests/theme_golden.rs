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
        assert!(
            ext.is_empty() || ext.starts_with('.'),
            "not an extension: {ext}"
        );
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
fn mime_parity_is_bounded_to_path_extensions() {
    let fixtures = cases("mime-boundary");
    let mut object_seen = false;
    let mut function_seen = false;
    for case in fixtures {
        let ext = case["input"]["extension"].as_str().unwrap();
        let expected = &case["expected"];
        match case["contract"].as_str().unwrap() {
            "path-extension" => {
                assert!(ext.is_empty() || ext.starts_with('.'));
                assert_eq!(expected["type"], "string");
                assert_eq!(get_mime_type(ext), expected["value"].as_str().unwrap());
            }
            "outside-extension-contract" => {
                assert!(!ext.is_empty() && !ext.starts_with('.'));
                // This is Rust's total-function fallback, NOT a TS parity claim.
                assert_eq!(get_mime_type(ext), "application/octet-stream");
                match expected["type"].as_str().unwrap() {
                    "object" => {
                        assert_eq!(ext.to_lowercase(), "__proto__");
                        assert_eq!(expected["identity"], "Object.prototype");
                        object_seen = true;
                    }
                    "function" => {
                        assert_eq!(ext.to_lowercase(), "constructor");
                        assert_eq!(expected["identity"], "Object");
                        function_seen = true;
                    }
                    "string" => assert_eq!(expected["value"], "application/octet-stream"),
                    other => panic!("unexpected MIME observation: {other}"),
                }
            }
            other => panic!("unknown MIME contract: {other}"),
        }
    }
    assert!(
        object_seen && function_seen,
        "prototype-name evidence is missing"
    );
}

#[test]
fn version_comparison_matches_real_library_source() {
    let mut positive_overflow_seen = false;
    let mut negative_overflow_seen = false;
    for case in cases("versions") {
        let input = &case["input"];
        let actual = compare_theme_version(&input["left"], &input["right"]);
        let expected = &case["expected"];
        match expected["kind"].as_str().expect("type-aware number oracle") {
            "finite" => {
                assert!(actual.is_finite(), "{}: {input}", case["name"]);
                assert_eq!(
                    actual,
                    expected["value"].as_f64().unwrap(),
                    "{}: {input}",
                    case["name"]
                );
            }
            "nan" => assert!(actual.is_nan(), "{}: {input}", case["name"]),
            "+infinity" => {
                assert_eq!(actual, f64::INFINITY, "{}: {input}", case["name"]);
                positive_overflow_seen = true;
            }
            "-infinity" => {
                assert_eq!(actual, f64::NEG_INFINITY, "{}: {input}", case["name"]);
                negative_overflow_seen = true;
            }
            other => panic!("unknown numeric observation: {other}"),
        }
    }
    assert!(
        positive_overflow_seen && negative_overflow_seen,
        "overflow cases are missing"
    );
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
