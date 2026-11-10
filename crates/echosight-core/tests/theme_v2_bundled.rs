use echosight_core::theme::v2::{ConversionInput, State, Theme, convert_v1};
use serde_json::Value;
use std::{fs, path::PathBuf};

#[test]
fn all_eleven_source_copies_convert_validate_resolve_and_roundtrip() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/theme_v2_fixtures");
    let original = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/themes");
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(root.join("manifest.json")).unwrap()).unwrap();
    let themes = manifest["themes"].as_array().unwrap();
    assert_eq!(themes.len(), 11);
    let mut provisional = 0;
    for entry in themes {
        let id = entry["id"].as_str().unwrap();
        let folder = root.join(id);
        let bytes = fs::read(folder.join("source.json")).unwrap();
        assert_eq!(
            bytes,
            fs::read(original.join(entry["source"].as_str().unwrap())).unwrap(),
            "source drift {id}"
        );
        let source: Value = serde_json::from_slice(&bytes).unwrap();
        let css = entry["css"].as_str().map(|path| {
            let bytes = fs::read(folder.join("source.css")).unwrap();
            assert_eq!(
                bytes,
                fs::read(original.join(path)).unwrap(),
                "CSS source drift {id}"
            );
            String::from_utf8(bytes).unwrap()
        });
        let assets: Vec<String> = serde_json::from_value(entry["assets"].clone()).unwrap();
        let input = ConversionInput {
            document: &source,
            css_file: css.as_deref(),
            asset_files: &assets,
        };
        let conversion = convert_v1(input).unwrap_or_else(|d| panic!("{id}: {d:#?}"));
        assert!(
            !conversion
                .diagnostics
                .iter()
                .any(|d| d.code == "malformed-declaration"
                    && d.value.as_str().is_some_and(|s| s.trim().is_empty())),
            "valid empty declaration reported as malformed in {id}"
        );
        assert_eq!(
            conversion,
            convert_v1(input).unwrap(),
            "nondeterminism {id}"
        );
        assert_eq!(conversion.theme["id"], source["id"]);
        assert_eq!(conversion.theme["version"], source["version"]);
        assert!(conversion.theme.get("cssFile").is_none());
        assert!(conversion.theme.get("customCSS").is_none());
        let theme = Theme::parse(&conversion.theme).unwrap();
        for slot in theme.slot_names() {
            for opacity in [0., 0.5, 1.] {
                assert!(
                    theme.slot(slot, &State::PRECEDENCE, opacity).is_some(),
                    "{id} {slot}"
                );
            }
        }
        for slot in [
            "dialog.scrim",
            "dialog.window",
            "dialog.title",
            "dialog.text",
            "dialog.hint",
            "dialog.button.primary",
            "dialog.button.secondary",
        ] {
            assert!(theme.slot(slot, &[], 1.).is_some(), "{id} {slot}");
        }
        let encoded = serde_json::to_string_pretty(&conversion).unwrap() + "\n";
        let roundtrip: echosight_core::theme::v2::Conversion =
            serde_json::from_str(&encoded).unwrap();
        assert_eq!(conversion, roundtrip);
        assert!(Theme::parse(&roundtrip.theme).is_ok());
        provisional += usize::from(conversion.provisional);
        if std::env::var_os("UPDATE_THEME_V2_FIXTURES").is_some() {
            fs::write(folder.join("converted.json"), &encoded).unwrap();
            fs::write(
                folder.join("CONVERSION-REPORT.md"),
                conversion.report_markdown(),
            )
            .unwrap();
        } else {
            assert_eq!(
                encoded,
                fs::read_to_string(folder.join("converted.json"))
                    .expect("freeze reviewed fixture via UPDATE_THEME_V2_FIXTURES=1"),
                "conversion drift {id}"
            );
            assert_eq!(
                conversion.report_markdown(),
                fs::read_to_string(folder.join("CONVERSION-REPORT.md")).unwrap()
            );
        }
        println!(
            "{id}: valid, {} diagnostics, provisional={}",
            conversion.diagnostics.len(),
            conversion.provisional
        );
        assert_eq!(
            bytes,
            fs::read(original.join(entry["source"].as_str().unwrap())).unwrap()
        );
    }
    assert_eq!(provisional, 4);
}
