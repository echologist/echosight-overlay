use echosight_core::settings::convert_hotkey_format;
use echosight_platform::hotkeys::accelerator::{
    AcceleratorError, AcceleratorTarget as Target, MappingIssue, electron_accelerator,
};
use global_hotkey::hotkey::{Code, HotKey, Modifiers as Mods};

fn chord(text: &str, target: Target, mods: Mods, key: Code) {
    assert_eq!(
        electron_accelerator(text, target),
        Ok(HotKey::new(Some(mods), key)),
        "{target:?}: {text:?}"
    );
}

#[test]
fn command_or_control_target_mapping_host_independent() {
    for (target, primary) in [
        (Target::Windows, Mods::CONTROL),
        (Target::Linux, Mods::CONTROL),
        (Target::Mac, Mods::META),
    ] {
        for alias in [
            "CommandOrControl",
            "CmdOrCtrl",
            "cOmMaNdOrCoNtRoL",
            "CMDORCTRL",
        ] {
            chord(&format!("{alias}+A"), target, primary, Code::KeyA);
        }
        for not_alias in ["CommandOrCtrl", "CmdOrControl"] {
            chord(&format!("{not_alias}+A"), target, Mods::empty(), Code::KeyA);
            assert!(
                electron_accelerator(&format!("A+{not_alias}"), target)
                    .unwrap_err()
                    .is_conversion_failure()
            );
        }
    }
}

#[test]
fn unknown_prefix_overwritten_unknown_suffix_rejected() {
    chord("Bogus+A", Target::Windows, Mods::empty(), Code::KeyA);
    assert_eq!(
        electron_accelerator("A+Bogus", Target::Windows),
        Err(AcceleratorError::MissingOrUnknownKey)
    );
}

#[test]
fn last_key_wins_control_after_key_and_empty_segments() {
    chord(
        "+A++B+Ctrl+Control+",
        Target::Windows,
        Mods::CONTROL,
        Code::KeyB,
    );
}

#[test]
fn implicit_shift_accumulates_across_replaced_keys() {
    chord("!+Bogus+A", Target::Windows, Mods::SHIFT, Code::KeyA);
    chord(
        "Plus+Shift+!+Ctrl+B",
        Target::Windows,
        Mods::SHIFT | Mods::CONTROL,
        Code::KeyB,
    );
}

#[test]
fn named_plus_is_shift_equal_literal_plus_is_not_a_key() {
    chord("Plus", Target::Windows, Mods::SHIFT, Code::Equal);
    chord("numadd", Target::Windows, Mods::empty(), Code::NumpadAdd);
    for text in ["+", "Ctrl++", "++", "Ctrl", ""] {
        assert_eq!(
            electron_accelerator(text, Target::Windows),
            Err(AcceleratorError::MissingOrUnknownKey)
        );
    }
}

#[test]
fn non_ascii_is_checked_before_trim_or_unknown_overwrite() {
    for text in ["\u{a0}A\u{a0}", "\u{feff}A", "é+A", "A+\u{85}"] {
        assert_eq!(
            electron_accelerator(text, Target::Windows),
            Err(AcceleratorError::NonAscii)
        );
    }
    let raw = "\u{a0}A\u{a0}";
    let normalized = convert_hotkey_format(Some(&raw.into()), "fallback");
    assert_eq!(normalized, "A");
    chord(&normalized, Target::Windows, Mods::empty(), Code::KeyA);
    assert_eq!(raw, "\u{a0}A\u{a0}");
}

#[test]
fn ascii_trim_matches_chromium_exactly_including_vertical_tab() {
    for c in 0u8..=127 {
        let text = format!("{}A{}", char::from(c), char::from(c));
        let accepted = matches!(c, 0x09..=0x0d | 0x20 | b'+');
        assert_eq!(
            electron_accelerator(&text, Target::Windows).is_ok(),
            accepted,
            "byte {c}"
        );
    }
    chord(
        "\t\n\x0b\x0c\r Ctrl \t+\x0b A \r\n",
        Target::Windows,
        Mods::CONTROL,
        Code::KeyA,
    );
}

#[test]
fn cased_main_fastpath_preserves_residual_cmd_windows_win_flag() {
    for (raw, mods) in [
        ("Cmd+A", Mods::CONTROL),
        ("Command+A", Mods::CONTROL),
        ("CommandOrControl+Cmd+A", Mods::CONTROL | Mods::META),
        ("commandorcontrol+Cmd+A", Mods::CONTROL),
    ] {
        let original = raw.to_owned();
        let normalized = convert_hotkey_format(Some(&raw.into()), "fallback");
        chord(&normalized, Target::Windows, mods, Code::KeyA);
        assert_eq!(raw, original);
    }
    chord("Cmd+A", Target::Windows, Mods::META, Code::KeyA);
}

#[test]
fn altgr_is_valid_but_unresolved_not_ctrl_alt_or_conversion_failure() {
    for target in [Target::Windows, Target::Linux, Target::Mac] {
        let error = electron_accelerator("AltGr+A", target).unwrap_err();
        assert_eq!(
            error,
            AcceleratorError::UnresolvedPlatformToken {
                target,
                token: "AltGr"
            }
        );
        assert!(!error.is_conversion_failure());
        assert!(
            electron_accelerator("AltGr+Bogus", target)
                .unwrap_err()
                .is_conversion_failure()
        );
        assert!(
            electron_accelerator("AltGr", target)
                .unwrap_err()
                .is_conversion_failure()
        );
    }
}

#[test]
fn backend_mapping_diagnostics_are_not_conversion_failures() {
    for (target, text, key, issue) in [
        (
            Target::Linux,
            "MediaPlayPause",
            Code::MediaPlayPause,
            MappingIssue::BackendUnsupported,
        ),
        (
            Target::Mac,
            "F24",
            Code::F24,
            MappingIssue::BackendUnsupported,
        ),
        (
            Target::Mac,
            "ScrollLock",
            Code::ScrollLock,
            MappingIssue::BackendUnsupported,
        ),
        (
            Target::Mac,
            "MediaStop",
            Code::MediaStop,
            MappingIssue::BackendUnsupported,
        ),
        (
            Target::Mac,
            "PrintScreen",
            Code::PrintScreen,
            MappingIssue::DifferentNativeKey,
        ),
        (
            Target::Mac,
            "NumLock",
            Code::NumLock,
            MappingIssue::DifferentNativeKey,
        ),
        (
            Target::Mac,
            "VolumeUp",
            Code::AudioVolumeUp,
            MappingIssue::UnprovenPlatformPolicy,
        ),
    ] {
        let error = electron_accelerator(text, target).unwrap_err();
        assert_eq!(
            error,
            AcceleratorError::UnresolvedKeyMapping { target, key, issue }
        );
        assert!(!error.is_conversion_failure());
        chord(&format!("{text}+A"), target, Mods::empty(), Code::KeyA);
    }
}

#[test]
fn duplicate_identities_use_resolved_key_and_modifiers() {
    for (a, b) in [
        ("Plus", "Shift+="),
        ("Ctrl+!", "Ctrl+Shift+1"),
        ("Return", "Enter"),
        ("Meta+A", "Super+A"),
        ("Ctrl+A", "A+Ctrl+Control"),
        ("!+A", "Shift+A"),
    ] {
        let a = electron_accelerator(a, Target::Windows).unwrap();
        let b = electron_accelerator(b, Target::Windows).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.id(), b.id());
    }
}

#[test]
fn named_shift_controls_reject_the_defective_all_true_csv_column() {
    for (text, key) in [
        ("Enter", Code::Enter),
        ("F1", Code::F1),
        ("Space", Code::Space),
        ("numadd", Code::NumpadAdd),
        ("VolumeUp", Code::AudioVolumeUp),
    ] {
        let actual = electron_accelerator(text, Target::Windows).unwrap();
        assert_eq!(actual, HotKey::new(None, key));
        assert_ne!(
            actual,
            HotKey::new(Some(Mods::SHIFT), key),
            "bad implied_shift=True must fail: {text}"
        );
    }
    chord("Plus", Target::Windows, Mods::SHIFT, Code::Equal);
}
