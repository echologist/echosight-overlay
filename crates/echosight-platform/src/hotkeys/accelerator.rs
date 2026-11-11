//! Electron 41.3.0 accelerator conversion, not native registration.
//!
//! Call after `echosight_core::settings::convert_hotkey_format`; never use this
//! to rewrite persisted raw text or for renderer/display duplicate comparison.
//! Pinned source/hash provenance and independently observed parser acceptance
//! live in `tests/accelerator_fixture`. In particular, the research CSV's named
//! `implied_shift` column is defective; only named `Plus` implies Shift.
//!
//! `Ok(HotKey)` establishes a code/modifier identity, NOT OS acceptance. Repeat
//! policy is outside this adapter: global-hotkey 0.10 adds MOD_NOREPEAT on
//! Windows, unlike the traced Electron path. Linux layout/Wayland and Mac native
//! behavior remain unverified. Do not instantiate a native manager here.
use global_hotkey::hotkey::{Code, HotKey, Modifiers};

/// Parser target, independent of the host compiling or running the adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceleratorTarget {
    Windows,
    Linux,
    Mac,
}

/// A valid Electron key whose native mapping cannot yet be preserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingIssue {
    /// The pinned backend has no mapping for the corresponding `Code`.
    BackendUnsupported,
    /// The backend would choose a different native key from pinned Chromium.
    DifferentNativeKey,
    /// Native media policy differs or has not been established for this target.
    UnprovenPlatformPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AcceleratorError {
    #[error("Electron accelerator input must be ASCII before trimming")]
    NonAscii,
    #[error("Electron accelerator has no known final key")]
    MissingOrUnknownKey,
    #[error("{token} is syntactically valid but unresolved on {target:?}")]
    UnresolvedPlatformToken {
        target: AcceleratorTarget,
        token: &'static str,
    },
    #[error("{key:?} mapping on {target:?} is unresolved: {issue:?}")]
    UnresolvedKeyMapping {
        target: AcceleratorTarget,
        key: Code,
        issue: MappingIssue,
    },
}

impl AcceleratorError {
    /// True only for Electron conversion failures, which abort its outer
    /// registration loop. Other errors are inactive adapter diagnostics, not
    /// observed Electron OS false results. A future registry must distinguish
    /// these cases; this module performs no registration or continuation.
    pub fn is_conversion_failure(&self) -> bool {
        matches!(self, Self::NonAscii | Self::MissingOrUnknownKey)
    }
}

/// Convert main-normalized text with Electron's tokenizer and key semantics.
///
/// Callers can compare returned `HotKey`s (or their IDs) for resolved identity
/// within one target. Registration success/ownership and retries are separate.
/// An unknown prefix is allowed when a later key replaces it; unknown final
/// keys fail even when a modifier follows. AltGr is accepted syntactically but
/// remains unresolved on Windows, and Linux/Mac policy is also unproven.
pub fn electron_accelerator(
    main_normalized: &str,
    target: AcceleratorTarget,
) -> Result<HotKey, AcceleratorError> {
    // accelerator_util.cc checks the WHOLE string before tokenization.
    if !main_normalized.is_ascii() {
        return Err(AcceleratorError::NonAscii);
    }
    let mut modifiers = Modifiers::empty();
    let mut key = None;
    let mut altgr = false;
    for token in main_normalized.split('+').map(|token| {
        // Chromium string_split_internal.h selects kWhitespaceASCII for char.
        // string_util_constants.cc: 09, 0B, 0C, 20, 0A, 0D. Rust's ASCII
        // whitespace helper omits 0B; Unicode trim would accept too much.
        token.trim_matches(['\t', '\x0b', '\x0c', ' ', '\n', '\r'])
    }) {
        if token.is_empty() {
            continue;
        }
        let token = token.to_ascii_lowercase();
        match token.as_str() {
            "shift" => modifiers |= Modifiers::SHIFT,
            "control" | "ctrl" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            // Electron's applied command-ismediakey.patch preserves COMMAND
            // as MOD_WIN on Windows. The main normalizer (not this parser)
            // normally rewrites Cmd, except its exact-case primary fast path.
            "cmd" | "command" | "meta" | "super" => modifiers |= Modifiers::META,
            "commandorcontrol" | "cmdorctrl" => {
                modifiers |= if target == AcceleratorTarget::Mac {
                    Modifiers::META
                } else {
                    Modifiers::CONTROL
                };
            }
            "altgr" => altgr = true,
            _ => {
                let (next_key, shifted) = key_from_token(&token);
                key = next_key; // Including UNKNOWN: every nonmodifier overwrites.
                if shifted {
                    modifiers |= Modifiers::SHIFT; // Never cleared by a later key.
                }
            }
        }
    }
    // Conversion failure takes priority over valid-but-unresolved diagnostics.
    let key = key.ok_or(AcceleratorError::MissingOrUnknownKey)?;
    if altgr {
        return Err(AcceleratorError::UnresolvedPlatformToken {
            target,
            token: "AltGr",
        });
    }
    if let Some(issue) = mapping_issue(key, target) {
        return Err(AcceleratorError::UnresolvedKeyMapping { target, key, issue });
    }
    Ok(HotKey::new(Some(modifiers), key))
}

fn mapping_issue(key: Code, target: AcceleratorTarget) -> Option<MappingIssue> {
    use AcceleratorTarget::{Linux, Mac};
    use Code::*;
    use MappingIssue::*;
    match (target, key) {
        // The X11 backend has MediaPlay and MediaPause but no PlayPause arm.
        (Linux, MediaPlayPause) => Some(BackendUnsupported),
        (Mac, F21 | F22 | F23 | F24 | ScrollLock | MediaStop) => Some(BackendUnsupported),
        // Chromium's MacKeyCodeForWindowsKeyCode returns -1, while the Rust
        // backend chooses 0x46 (PrintScreen) / 0x47 (NumLock). No substitution.
        (Mac, PrintScreen | NumLock) => Some(DifferentNativeKey),
        // Electron's patched media-listener path and Rust's Carbon/media split
        // are not interchangeable merely because Code has a similar name.
        (
            Mac,
            AudioVolumeUp | AudioVolumeDown | AudioVolumeMute | MediaPlayPause | MediaTrackNext
            | MediaTrackPrevious,
        ) => Some(UnprovenPlatformPolicy),
        _ => None,
    }
}

/// Pinned keyboard_util.cc's lookup and character switch, joined to the
/// Windows backend's exact VKEY/VK identity. No Rust accelerator parser aliases.
fn key_from_token(token: &str) -> (Option<Code>, bool) {
    use Code::*;
    let (key, shift) = match token {
        "a" => (KeyA, false),
        "b" => (KeyB, false),
        "c" => (KeyC, false),
        "d" => (KeyD, false),
        "e" => (KeyE, false),
        "f" => (KeyF, false),
        "g" => (KeyG, false),
        "h" => (KeyH, false),
        "i" => (KeyI, false),
        "j" => (KeyJ, false),
        "k" => (KeyK, false),
        "l" => (KeyL, false),
        "m" => (KeyM, false),
        "n" => (KeyN, false),
        "o" => (KeyO, false),
        "p" => (KeyP, false),
        "q" => (KeyQ, false),
        "r" => (KeyR, false),
        "s" => (KeyS, false),
        "t" => (KeyT, false),
        "u" => (KeyU, false),
        "v" => (KeyV, false),
        "w" => (KeyW, false),
        "x" => (KeyX, false),
        "y" => (KeyY, false),
        "z" => (KeyZ, false),
        "0" => (Digit0, false),
        "1" => (Digit1, false),
        "2" => (Digit2, false),
        "3" => (Digit3, false),
        "4" => (Digit4, false),
        "5" => (Digit5, false),
        "6" => (Digit6, false),
        "7" => (Digit7, false),
        "8" => (Digit8, false),
        "9" => (Digit9, false),
        "!" => (Digit1, true),
        "@" => (Digit2, true),
        "#" => (Digit3, true),
        "$" => (Digit4, true),
        "%" => (Digit5, true),
        "^" => (Digit6, true),
        "&" => (Digit7, true),
        "*" => (Digit8, true),
        "(" => (Digit9, true),
        ")" => (Digit0, true),
        "-" => (Minus, false),
        "_" => (Minus, true),
        "=" => (Equal, false),
        "plus" => (Equal, true),
        "[" => (BracketLeft, false),
        "{" => (BracketLeft, true),
        "]" => (BracketRight, false),
        "}" => (BracketRight, true),
        "\\" => (Backslash, false),
        "|" => (Backslash, true),
        ";" => (Semicolon, false),
        ":" => (Semicolon, true),
        "'" => (Quote, false),
        "\"" => (Quote, true),
        "," => (Comma, false),
        "<" => (Comma, true),
        "." => (Period, false),
        ">" => (Period, true),
        "/" => (Slash, false),
        "?" => (Slash, true),
        "`" => (Backquote, false),
        "~" => (Backquote, true),
        "backspace" | "\x08" => (Backspace, false),
        "delete" | "\x7f" => (Delete, false),
        "escape" | "esc" | "\x1b" => (Escape, false),
        "tab" => (Tab, false),
        "space" => (Space, false),
        "enter" | "return" => (Enter, false),
        "capslock" => (CapsLock, false),
        "numlock" => (NumLock, false),
        "scrolllock" => (ScrollLock, false),
        "up" => (ArrowUp, false),
        "down" => (ArrowDown, false),
        "left" => (ArrowLeft, false),
        "right" => (ArrowRight, false),
        "home" => (Home, false),
        "end" => (End, false),
        "pageup" => (PageUp, false),
        "pagedown" => (PageDown, false),
        "insert" => (Insert, false),
        "printscreen" => (PrintScreen, false),
        "num0" => (Numpad0, false),
        "num1" => (Numpad1, false),
        "num2" => (Numpad2, false),
        "num3" => (Numpad3, false),
        "num4" => (Numpad4, false),
        "num5" => (Numpad5, false),
        "num6" => (Numpad6, false),
        "num7" => (Numpad7, false),
        "num8" => (Numpad8, false),
        "num9" => (Numpad9, false),
        "numadd" => (NumpadAdd, false),
        "numdec" => (NumpadDecimal, false),
        "numdiv" => (NumpadDivide, false),
        "nummult" => (NumpadMultiply, false),
        "numsub" => (NumpadSubtract, false),
        "volumeup" => (AudioVolumeUp, false),
        "volumedown" => (AudioVolumeDown, false),
        "volumemute" => (AudioVolumeMute, false),
        "medianexttrack" => (MediaTrackNext, false),
        "mediaprevioustrack" => (MediaTrackPrevious, false),
        "mediaplaypause" => (MediaPlayPause, false),
        "mediastop" => (MediaStop, false),
        "f1" => (F1, false),
        "f2" => (F2, false),
        "f3" => (F3, false),
        "f4" => (F4, false),
        "f5" => (F5, false),
        "f6" => (F6, false),
        "f7" => (F7, false),
        "f8" => (F8, false),
        "f9" => (F9, false),
        "f10" => (F10, false),
        "f11" => (F11, false),
        "f12" => (F12, false),
        "f13" => (F13, false),
        "f14" => (F14, false),
        "f15" => (F15, false),
        "f16" => (F16, false),
        "f17" => (F17, false),
        "f18" => (F18, false),
        "f19" => (F19, false),
        "f20" => (F20, false),
        "f21" => (F21, false),
        "f22" => (F22, false),
        "f23" => (F23, false),
        "f24" => (F24, false),
        _ => return (None, false),
    };
    (Some(key), shift)
}
