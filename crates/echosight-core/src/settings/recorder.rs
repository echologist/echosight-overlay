//! Pure recorder state. The shell executes ordered effects; this module has no
//! timer, DOM, global registration, persistence or platform dependency.
use serde_json::Value;

use super::{HotkeyAction, KeyEvent, find_duplicate_hotkey_action, hotkey_recording_result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordingEffect {
    /// Display "Press keys now...", mark the input, focus it, label button Cancel.
    Begin {
        action: HotkeyAction,
    },
    /// Clear recording styles, label button Record; Some restores display text.
    End {
        action: HotkeyAction,
        restore: Option<String>,
    },
    SuspendGlobalHotkeys,
    /// Re-register the shell's currently committed shortcuts (not an implicit save).
    ReregisterGlobalHotkeys,
    /// Prevent default and stop propagation, even for ignored/invalid keys.
    ConsumeKeydown,
    Alert(String),
    /// Write control text and update in-memory settings. No persistence yet.
    Recorded {
        action: HotkeyAction,
        hotkey: String,
    },
    /// Call finish_deferred_stop after this delay. Each success queues a callback.
    ScheduleStop {
        delay_ms: u64,
    },
}

#[derive(Clone, Debug)]
struct Recording {
    action: HotkeyAction,
    previous_display: String,
}

#[derive(Clone, Debug, Default)]
pub struct HotkeyRecorder {
    recording: Option<Recording>,
}

impl HotkeyRecorder {
    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    pub fn action(&self) -> Option<HotkeyAction> {
        self.recording.as_ref().map(|recording| recording.action)
    }

    /// Pass None when the shell cannot begin (missing/removed input or button).
    /// Invalid actions are ignored; clicking the current action cancels; switching
    /// actions restores the old control and resumes then suspends global hotkeys.
    pub fn record(&mut self, action: &str, previous_display: Option<&str>) -> Vec<RecordingEffect> {
        let Some(action) = HotkeyAction::parse(action) else {
            return Vec::new();
        };
        if self.action() == Some(action) {
            return self.stop();
        }
        let mut effects = self.stop();
        if let Some(previous_display) = previous_display {
            self.recording = Some(Recording {
                action,
                previous_display: previous_display.into(),
            });
            effects.push(RecordingEffect::Begin { action });
            effects.push(RecordingEffect::SuspendGlobalHotkeys);
        }
        effects
    }

    pub fn stop(&mut self) -> Vec<RecordingEffect> {
        self.end(true)
    }

    /// Matches the uncancelled TS timeout: it stops whichever action is active
    /// when fired, not necessarily the one that originally scheduled it.
    pub fn finish_deferred_stop(&mut self) -> Vec<RecordingEffect> {
        self.end(false)
    }

    fn end(&mut self, restore_previous: bool) -> Vec<RecordingEffect> {
        let Some(recording) = self.recording.take() else {
            return Vec::new();
        };
        vec![
            RecordingEffect::End {
                action: recording.action,
                restore: restore_previous.then_some(recording.previous_display),
            },
            RecordingEffect::ReregisterGlobalHotkeys,
        ]
    }

    pub fn handle_keydown(
        &mut self,
        event: &KeyEvent,
        hotkeys: Option<&Value>,
        platform: &str,
    ) -> Vec<RecordingEffect> {
        let Some(action) = self.action() else {
            return Vec::new();
        };
        let mut effects = vec![RecordingEffect::ConsumeKeydown];
        let Some(result) = hotkey_recording_result(event, platform) else {
            return effects;
        };
        if let Some(warning) = result.warning {
            effects.push(RecordingEffect::Alert(warning));
        }
        if let Some(hotkey) = result.hotkey {
            if let Some(duplicate) = hotkeys.and_then(|hotkeys| {
                find_duplicate_hotkey_action(hotkeys, action, &hotkey, platform)
            }) {
                effects.push(RecordingEffect::Alert(format!(
                    "\"{hotkey}\" is already assigned to {}. Choose a different shortcut.",
                    duplicate.label()
                )));
                return effects;
            }
            effects.push(RecordingEffect::Recorded { action, hotkey });
            effects.push(RecordingEffect::ScheduleStop { delay_ms: 200 });
        }
        effects
    }
}
