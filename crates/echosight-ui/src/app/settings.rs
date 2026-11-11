//! Renderer settings restoration. Storage remains authoritative for load blocks.
use super::*;
use echosight_core::{
    settings::{
        HotkeyAction, HotkeyRecorder, HotkeySettings, KeyEvent, RecordingEffect, Settings,
        format_hotkey_for_display, migrate_settings, normalize_accelerator_hotkeys,
        normalize_settings, renderer_defaults,
    },
    storage::LoadOrigin,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SettingsOrigin {
    Missing,
    ExplicitNull,
    Loaded,
    Recovered,
    LoadErr,
    CoercionErr,
}
#[derive(Debug)]
pub(super) struct SettingsLoadOutcome {
    pub settings: Settings,
    pub committed: HotkeySettings,
    pub origin: SettingsOrigin,
    pub load_error: Option<String>,
    pub coercion_error: Option<String>,
    pub persisted: Option<Result<(), String>>,
    pub save_blocked: bool,
}

pub(super) fn platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "MacIntel"
    } else {
        "Linux"
    }
}

pub(super) fn load_settings(storage: &Storage, platform: &str) -> SettingsLoadOutcome {
    let mut outcome = SettingsLoadOutcome {
        settings: renderer_defaults(platform),
        committed: HotkeySettings::default(),
        origin: SettingsOrigin::LoadErr,
        load_error: None,
        coercion_error: None,
        persisted: None,
        save_blocked: false,
    };
    let loaded = match storage.load_with_origin(FileKind::Settings) {
        Ok(loaded) => loaded,
        Err(error) => {
            outcome.load_error = Some(error.to_string());
            outcome.save_blocked = true;
            return outcome;
        }
    };
    outcome.committed =
        normalize_accelerator_hotkeys(&loaded.value["hotkeys"], &HotkeySettings::default());
    outcome.origin = match loaded.origin {
        LoadOrigin::Missing => SettingsOrigin::Missing,
        LoadOrigin::Recovered => SettingsOrigin::Recovered,
        LoadOrigin::Existing if loaded.value.is_null() => SettingsOrigin::ExplicitNull,
        LoadOrigin::Existing => SettingsOrigin::Loaded,
    };
    match migrate_settings(&loaded.value, &outcome.settings) {
        Ok(migration) => {
            outcome.settings = migration.settings;
            if migration.changed {
                outcome.persisted = Some(
                    storage
                        .save(
                            FileKind::Settings,
                            &serde_json::to_value(&outcome.settings).expect("finite settings"),
                        )
                        .map_err(|e| e.to_string()),
                );
            }
        }
        Err(error) => {
            outcome.origin = SettingsOrigin::CoercionErr;
            outcome.coercion_error = Some(error.to_string());
        }
    }
    outcome
}

#[derive(Clone, Copy)]
pub(super) enum SettingsCommand {
    Open,
    Record(HotkeyAction),
    Reset,
    SoundEnabled,
    Transparency(u8),
    Volume(u8),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecorderDecodeError {
    SnapshotFailed(String),
    UnrepresentableKey(String),
    UnsupportedTranslation(String),
}
pub(super) struct SettingsWrite {
    pub value: Value,
    pub hotkeys: HotkeySettings,
}
pub(super) struct SettingsState {
    pub loading: bool,
    pub outcome: Option<SettingsLoadOutcome>,
    pub live: Settings,
    pub committed: HotkeySettings,
    pub displayed: HotkeySettings,
    pub recorder: HotkeyRecorder,
    pub pending: VecDeque<SettingsWrite>,
    pub saving: bool,
    pub notice: Option<String>,
    // A failed Save must remain visible even after Settings has been closed.
    pub save_error: Option<String>,
    pub range_bounds: [Option<gpui::Bounds<gpui::Pixels>>; 2],
}
impl Default for SettingsState {
    fn default() -> Self {
        let live = renderer_defaults(platform());
        Self {
            loading: true,
            outcome: None,
            displayed: live.hotkeys.clone(),
            live,
            committed: HotkeySettings::default(),
            recorder: HotkeyRecorder::default(),
            pending: VecDeque::new(),
            saving: false,
            notice: None,
            save_error: None,
            range_bounds: [None, None],
        }
    }
}
impl TaskApp {
    pub(super) fn settings_command(
        &mut self,
        command: SettingsCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.settings.loading || self.overlay.quit_requested {
            return;
        }
        if matches!(command, SettingsCommand::Open) {
            if self.dialog.is_some() {
                return;
            }
            self.restore_focus = window.focused(cx);
            self.dialog = Some(Dialog::Settings);
            self.dragging = None;
            self.write_hotkey_controls();
            let focus = self
                .controls
                .entry("cancel".into())
                .or_insert_with(|| cx.focus_handle())
                .clone();
            window.focus(&focus, cx);
        } else if matches!(self.dialog, Some(Dialog::Settings)) {
            match command {
                SettingsCommand::Record(action) => {
                    let previous = self.settings.displayed.get(action).to_owned();
                    let effects = self
                        .settings
                        .recorder
                        .record(action.as_str(), Some(&previous));
                    self.recording_effects(effects, window, cx);
                }
                SettingsCommand::Reset => {
                    self.dialog = Some(Dialog::SettingsReset);
                    let focus = self
                        .controls
                        .entry("cancel".into())
                        .or_insert_with(|| cx.focus_handle())
                        .clone();
                    window.focus(&focus, cx);
                }
                SettingsCommand::SoundEnabled => {
                    self.settings.live.sounds.enabled = !self.settings.live.sounds.enabled
                }
                SettingsCommand::Transparency(value) => {
                    self.settings.live.transparency = value.min(100)
                }
                SettingsCommand::Volume(value) => self.settings.live.sounds.volume = value.min(100),
                SettingsCommand::Open => unreachable!(),
            }
        }
        cx.notify();
    }
    pub(super) fn write_hotkey_controls(&mut self) {
        for action in HotkeyAction::ALL {
            self.settings.displayed.set(
                action,
                format_hotkey_for_display(self.settings.live.hotkeys.get(action), platform()),
            );
        }
    }
    pub(super) fn accept_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.dialog, Some(Dialog::SettingsReset)) {
            self.settings.live.hotkeys = renderer_defaults(platform()).hotkeys;
            self.write_hotkey_controls();
            self.dialog = Some(Dialog::Settings);
            self.settings.notice =
                Some("Hotkeys reset to defaults. Click \"Save Settings\" to apply.".into());
            cx.notify();
            return;
        }
        if self.settings.loading {
            return;
        }
        self.stop_recording(window, cx);
        let mut controls = self.settings.live.clone();
        controls.hotkeys = self.settings.displayed.clone();
        // Read the displayed controls AFTER stop/restore, including stale placeholder text.
        self.settings.live = normalize_settings(
            &serde_json::to_value(controls).expect("finite controls"),
            &self.settings.live,
        )
        .expect("typed controls have no coercion errors");
        let write = SettingsWrite {
            value: serde_json::to_value(&self.settings.live).expect("finite settings"),
            hotkeys: self.settings.live.hotkeys.clone(),
        };
        self.settings.pending.push_back(write);
        self.pump_settings(window, cx);
    }
    pub(super) fn pump_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.loading || self.settings.saving {
            return;
        }
        let Some(write) = self.settings.pending.pop_front() else {
            return;
        };
        let storage = self.storage.clone();
        self.settings.saving = true;
        let task = cx.background_executor().spawn(async move {
            let result = storage
                .ok_or_else(|| "Settings storage unavailable".to_owned())
                .and_then(|s| {
                    s.save(FileKind::Settings, &write.value)
                        .map_err(|e| e.to_string())
                });
            (write, result)
        });
        cx.spawn_in(window, async move |this, cx| {
            let (write, result) = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.settings.saving = false;
                match result {
                    Ok(()) => {
                        let next = normalize_accelerator_hotkeys(
                            &serde_json::to_value(write.hotkeys).expect("hotkeys"),
                            &this.settings.committed,
                        );
                        if next != this.settings.committed {
                            this.settings.committed = next;
                            this.register_committed();
                        }
                        // No generation gate: source closes whichever Settings dialog is now open.
                        if matches!(this.dialog, Some(Dialog::Settings | Dialog::SettingsReset)) {
                            this.close_settings(window, cx);
                        }
                        this.status =
                            "Settings saved successfully! Hotkey changes are active now.".into();
                        this.settings.notice = Some(this.status.clone());
                        this.settings.save_error = None;
                    }
                    Err(error) => {
                        this.status = format!("Error saving settings: {error}. Please try again.");
                        this.settings.notice = Some(this.status.clone());
                        this.settings.save_error = Some(this.status.clone());
                    }
                }
                this.pump_settings(window, cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    pub(super) fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_recording(window, cx);
        self.dialog = None;
        self.form_error = None;
        let focus = self
            .restore_focus
            .take()
            .unwrap_or_else(|| self.input.focus_handle(cx));
        window.focus(&focus, cx);
        cx.notify();
    }
    pub(super) fn stop_recording(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.settings.recorder.stop();
        self.recording_effects(effects, window, cx);
    }
    /// Typed business-rule entry point for a future lossless native adapter.
    /// This does NOT decode GPUI key_char or certify native key consumption.
    /// The adapter must consume original input independently, including decode errors.
    pub fn recorder_input(
        &mut self,
        input: Result<KeyEvent, RecorderDecodeError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.settings.recorder.is_recording() {
            return false;
        }
        match input {
            Ok(event) => {
                let hotkeys = serde_json::to_value(&self.settings.live.hotkeys).expect("hotkeys");
                let effects =
                    self.settings
                        .recorder
                        .handle_keydown(&event, Some(&hotkeys), platform());
                self.recording_effects(effects, window, cx);
            }
            Err(error) => {
                self.settings.notice = Some(format!("Recorder input unavailable: {error:?}"));
                cx.notify();
            }
        }
        true
    }
    fn recording_effects(
        &mut self,
        effects: Vec<RecordingEffect>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for effect in effects {
            match effect {
                RecordingEffect::Begin { action } => {
                    self.settings
                        .displayed
                        .set(action, "Press keys now...".into());
                    let focus = self
                        .controls
                        .entry(format!("record-row-{}", action.as_str()))
                        .or_insert_with(|| cx.focus_handle())
                        .clone();
                    window.focus(&focus, cx);
                }
                RecordingEffect::End { action, restore } => {
                    if let Some(text) = restore {
                        self.settings.displayed.set(action, text);
                    }
                }
                RecordingEffect::SuspendGlobalHotkeys => {
                    if let Some(ports) = &self.overlay.ports {
                        let summary = ports.suspend_hotkeys();
                        if !summary.retained.is_empty() {
                            self.settings.notice = Some(format!(
                                "Hotkeys may remain reserved/unavailable: {:?}",
                                summary.retained
                            ));
                        }
                        eprintln!("hotkey suspension: {summary:?}");
                        self.overlay.release = Some(summary);
                    }
                }
                RecordingEffect::ReregisterGlobalHotkeys => self.register_committed(),
                RecordingEffect::ConsumeKeydown => {}
                RecordingEffect::Alert(message) => self.settings.notice = Some(message),
                RecordingEffect::Recorded { action, hotkey } => {
                    self.settings.displayed.set(action, hotkey.clone());
                    self.settings.live.hotkeys.set(action, hotkey);
                }
                RecordingEffect::ScheduleStop { delay_ms } => {
                    let timer = cx
                        .background_executor()
                        .timer(std::time::Duration::from_millis(delay_ms));
                    cx.spawn_in(window, async move |this, cx| {
                        timer.await;
                        let _ = this.update_in(cx, |this, window, cx| {
                            let effects = this.settings.recorder.finish_deferred_stop();
                            this.recording_effects(effects, window, cx);
                        });
                    })
                    .detach();
                }
            }
        }
        cx.notify();
    }
    pub(super) fn load_settings(&mut self, storage: Storage, cx: &mut Context<Self>) {
        let task = cx
            .background_executor()
            .spawn(async move { load_settings(&storage, platform()) });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |this, cx| {
                eprintln!(
                    "settings load: {:?}; blocked={}; load={:?}; coercion={:?}; migration={:?}",
                    outcome.origin,
                    outcome.save_blocked,
                    outcome.load_error,
                    outcome.coercion_error,
                    outcome.persisted
                );
                this.settings.loading = false;
                this.settings.live = outcome.settings.clone();
                this.settings.committed = outcome.committed.clone();
                this.settings.notice = outcome
                    .load_error
                    .clone()
                    .or(outcome.coercion_error.clone());
                this.settings.outcome = Some(outcome);
                this.maybe_register_initial();
                cx.notify();
            });
        })
        .detach();
    }
}
