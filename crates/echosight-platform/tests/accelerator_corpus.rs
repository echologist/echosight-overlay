use echosight_core::settings::convert_hotkey_format;
use echosight_platform::hotkeys::accelerator::{AcceleratorTarget as Target, electron_accelerator};
use global_hotkey::hotkey::{Code, HotKey, Modifiers as Mods};

mod fixture {
    use super::Code;
    include!("accelerator_fixture/observations.rs");
}

fn conversion_accepts(text: &str, target: Target) -> bool {
    match electron_accelerator(text, target) {
        Ok(_) => true,
        Err(error) => !error.is_conversion_failure(),
    }
}

#[test]
fn actual_electron_406_raw_and_main_normalized_pairs() {
    assert_eq!(fixture::PAIRS.len(), 406);
    for &(raw, normalized, raw_accepted, normalized_accepted) in fixture::PAIRS {
        let actual_normalized =
            convert_hotkey_format(Some(&raw.into()), "CommandOrControl+Shift+T");
        assert_eq!(actual_normalized, normalized, "main normalizer: {raw:?}");
        for target in [Target::Windows, Target::Linux, Target::Mac] {
            assert_eq!(
                conversion_accepts(raw, target),
                raw_accepted,
                "raw {target:?}: {raw:?}"
            );
            assert_eq!(
                conversion_accepts(normalized, target),
                normalized_accepted,
                "direct normalized {target:?}: {raw:?}"
            );
            assert_eq!(
                conversion_accepts(&actual_normalized, target),
                normalized_accepted,
                "raw through main {target:?}: {raw:?}"
            );
        }
    }
}

#[test]
fn actual_electron_upstream_cpp_and_helper_acceptance() {
    assert_eq!(fixture::CPP_CASES.len(), 9);
    assert_eq!(fixture::HELPER_ACCEPTED.len(), 2709);
    for target in [Target::Windows, Target::Linux, Target::Mac] {
        for &(text, accepted) in fixture::CPP_CASES {
            assert_eq!(conversion_accepts(text, target), accepted, "{text:?}");
        }
        for &text in fixture::HELPER_ACCEPTED {
            assert!(conversion_accepts(text, target), "{target:?} {text:?}");
        }
    }
}

#[test]
fn every_pinned_named_and_character_entry_matches_source_derived_windows_chord() {
    assert_eq!(fixture::SOURCE_TOKENS.len(), 154);
    let mut named_shift = Vec::new();
    for &(text, vk, code, shifted) in fixture::SOURCE_TOKENS {
        if text.len() > 1 && shifted {
            named_shift.push(text);
        }
        if ["+", " ", "\t", "\r"].contains(&text) {
            assert!(
                !conversion_accepts(text, Target::Windows),
                "unreachable character switch arm {text:?}"
            );
            continue;
        }
        let mods = if shifted { Mods::SHIFT } else { Mods::empty() };
        match code {
            Some(code) => {
                for spelling in [text.to_owned(), text.to_ascii_uppercase()] {
                    assert_eq!(
                        electron_accelerator(&spelling, Target::Windows),
                        Ok(HotKey::new(Some(mods), code)),
                        "{text:?} -> {vk}"
                    );
                }
            }
            None => {
                let expected = match vk {
                    "VKEY_MENU" => Mods::ALT,
                    "VKEY_COMMAND" => Mods::META,
                    "CommandOrControl" | "VKEY_CONTROL" => Mods::CONTROL,
                    "VKEY_SHIFT" => Mods::SHIFT,
                    "VKEY_ALTGR" => {
                        let error = electron_accelerator(&format!("{text}+A"), Target::Windows)
                            .unwrap_err();
                        assert!(!error.is_conversion_failure());
                        continue;
                    }
                    _ => panic!("unclassified source modifier {vk}"),
                };
                assert_eq!(
                    electron_accelerator(&format!("{text}+A"), Target::Windows),
                    Ok(HotKey::new(Some(expected), Code::KeyA))
                );
            }
        }
    }
    assert_eq!(
        named_shift,
        ["plus"],
        "the research CSV's all-true named shift column is invalid"
    );
}
