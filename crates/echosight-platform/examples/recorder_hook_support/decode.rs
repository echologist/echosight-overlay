//! Deliberately NOT a DOM decoder. Foreground-only allocation of typed errors.
use crate::policy::{Raw, SnapshotStatus};
use echosight_core::settings::KeyEvent;
#[derive(Debug, PartialEq, Eq)]
pub enum RecorderDecodeError {
    SnapshotFailed(String),
    UnrepresentableKey(String),
    UnsupportedTranslation(String),
}
pub fn decode(raw: &Raw) -> Result<KeyEvent, RecorderDecodeError> {
    if raw.status != SnapshotStatus::Ok || raw.hkl == 0 {
        return Err(RecorderDecodeError::SnapshotFailed(
            "Queue keyboard state/HKL unavailable".into(),
        ));
    }
    if !matches!(raw.message, 0x100 | 0x104) {
        return Err(RecorderDecodeError::UnrepresentableKey(
            "Not a recorder keydown".into(),
        ));
    }
    Err(RecorderDecodeError::UnsupportedTranslation(
        "Raw snapshot only: Chromium DOM/layout/dead-key equivalence unimplemented".into(),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_failure_never_becomes_a_successful_key() {
        assert!(matches!(
            decode(&Raw {
                status: SnapshotStatus::Failed,
                ..Raw::default()
            }),
            Err(RecorderDecodeError::SnapshotFailed(_))
        ));
        assert!(matches!(
            decode(&Raw {
                status: SnapshotStatus::MissingLayout,
                ..Raw::default()
            }),
            Err(RecorderDecodeError::SnapshotFailed(_))
        ));
    }
    #[test]
    fn even_plain_letters_are_not_claimed_as_dom_parity() {
        assert!(matches!(
            decode(&Raw {
                message: 0x100,
                vk: 65,
                hkl: 1,
                ..Raw::default()
            }),
            Err(RecorderDecodeError::UnsupportedTranslation(_))
        ));
    }
    #[test]
    fn releases_are_not_recorder_keydowns() {
        assert!(matches!(
            decode(&Raw {
                message: 0x101,
                vk: 65,
                hkl: 1,
                ..Raw::default()
            }),
            Err(RecorderDecodeError::UnrepresentableKey(_))
        ));
    }
}
