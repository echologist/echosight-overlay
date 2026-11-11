#[path = "settings_error_tests.rs"]
mod review_probes;

use super::overlay_tests::FakeOverlay;
use super::*;
use echosight_core::{
    settings::{HotkeyAction, KeyEvent, renderer_defaults},
    storage::{Stage, StorageHooks},
};
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Default)]
struct SettingsProbe {
    writes: Mutex<Vec<Value>>,
    fail: Mutex<Option<Stage>>,
}
impl StorageHooks for SettingsProbe {
    fn before(&self, stage: Stage, path: &std::path::Path) -> std::io::Result<()> {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let settings = name == "settings.json"
            || name.starts_with(".settings.json.")
            || name.starts_with("settings.json.corrupt-");
        if settings && self.fail.lock().unwrap().as_ref() == Some(&stage) {
            return Err(std::io::Error::other("injected settings fault"));
        }
        if settings && stage == Stage::Sync {
            self.writes
                .lock()
                .unwrap()
                .push(serde_json::from_slice(&std::fs::read(path)?).unwrap());
        }
        Ok(())
    }
}
fn storage(dir: &std::path::Path, probe: Arc<SettingsProbe>) -> Storage {
    Storage::with_hooks(DataPaths::new(dir).unwrap(), probe)
}
fn disk(dir: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("settings.json")).unwrap()).unwrap()
}
fn open_probed<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
    probe: Arc<SettingsProbe>,
) -> (Entity<TaskApp>, Rc<FakeOverlay>, &'a mut VisualTestContext) {
    cx.update(crate::components::text_input::init);
    let ports = Rc::new(FakeOverlay::default());
    ports.visible.set(true);
    let (scene, vc) = cx.add_window_view(|w, cx| {
        let mut app = TaskApp::with_storage(
            Ok((storage(dir, probe), DataPaths::new(dir).unwrap())),
            w,
            cx,
        );
        app.overlay.ports = Some(ports.clone());
        app.overlay.delay_elapsed = true;
        app
    });
    vc.simulate_resize(gpui::size(gpui::px(500.), gpui::px(700.)));
    vc.run_until_parked();
    (scene, ports, vc)
}
fn click(vc: &mut VisualTestContext, key: &'static str) {
    let bounds = vc
        .debug_bounds(key)
        .unwrap_or_else(|| panic!("missing {key}"));
    vc.simulate_click(bounds.center(), Default::default());
    vc.run_until_parked();
}
fn record(scene: &Entity<TaskApp>, vc: &mut VisualTestContext, action: HotkeyAction, key: &str) {
    scene.update_in(vc, |s, w, cx| {
        s.settings_command(SettingsCommand::Record(action), w, cx);
        assert!(s.recorder_input(
            Ok(KeyEvent {
                key: key.into(),
                ..Default::default()
            }),
            w,
            cx
        ));
    });
    vc.run_until_parked();
}
fn advance(vc: &mut VisualTestContext, ms: u64) {
    vc.background_executor
        .advance_clock(Duration::from_millis(ms));
    vc.run_until_parked();
}
use gpui::{TestAppContext, VisualTestContext};

fn open<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
) -> (Entity<TaskApp>, &'a mut VisualTestContext) {
    cx.update(crate::components::text_input::init);
    let (scene, cx) = cx.add_window_view(|w, cx| TaskApp::new(dir.into(), w, cx));
    cx.simulate_resize(gpui::size(gpui::px(500.), gpui::px(700.)));
    cx.run_until_parked();
    (scene, cx)
}

#[gpui::test]
fn missing_settings_persists_renderer_defaults_once(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let probe = Arc::new(SettingsProbe::default());
    let (_, _, _) = open_probed(cx, dir.path(), probe.clone());
    let (_, _, _) = open_probed(cx, dir.path(), probe.clone());
    assert_eq!(probe.writes.lock().unwrap().len(), 1);
    assert!(
        dir.path().join("settings.json").exists(),
        "renderer defaults must persist on missing settings"
    );
    let raw: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        raw,
        serde_json::to_value(echosight_core::settings::renderer_defaults(
            settings::platform()
        ))
        .unwrap()
    );
    assert!(!dir.path().join("current_tasks.json").exists());
    assert!(!dir.path().join("templates.json").exists());
}

#[gpui::test]
fn explicit_null_persists_renderer_defaults_once(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("settings.json"), "null").unwrap();
    let probe = Arc::new(SettingsProbe::default());
    let (scene, _, vc) = open_probed(cx, dir.path(), probe.clone());
    assert_eq!(
        scene.read_with(vc, |s, _| s.settings.outcome.as_ref().unwrap().origin),
        settings::SettingsOrigin::ExplicitNull
    );
    let (_, _, _) = open_probed(cx, dir.path(), probe.clone());
    assert_eq!(probe.writes.lock().unwrap().len(), 1);
    let raw: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("settings.json")).unwrap()).unwrap();
    assert!(
        raw.is_object(),
        "explicit null must become renderer defaults"
    );
}

#[gpui::test]
fn edit_target_removed_accept_closes_before_validation(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, vc) = open(cx, dir.path());
    scene.update_in(vc, |s, w, cx| {
        let id = s.state().unwrap().tasks()[0].id;
        s.activate(Command::Edit(id), w, cx);
        s.editor.update(cx, |e, cx| e.clear(w, cx));
        s.dispatch_background(BackgroundCommand::Delete(id), cx);
        s.activate(Command::Accept, w, cx);
        assert!(
            s.dialog.is_none(),
            "missing edit target closes before empty-text validation"
        );
    });
}

#[test]
fn load_error_blocks_save_without_write() {
    for content in ["null", "undefined", "NaN"] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), content).unwrap();
        let probe = Arc::new(SettingsProbe::default());
        if content == "null" {
            *probe.fail.lock().unwrap() = Some(Stage::Read);
        }
        let storage = storage(dir.path(), probe.clone());
        let outcome = settings::load_settings(&storage, "Linux");
        assert_eq!(outcome.origin, settings::SettingsOrigin::LoadErr);
        assert!(outcome.save_blocked);
        assert!(outcome.persisted.is_none());
        *probe.fail.lock().unwrap() = None;
        assert!(
            storage.save(FileKind::Settings, &Value::Null).is_err(),
            "actual Storage block survives cleared hook"
        );
        assert!(probe.writes.lock().unwrap().is_empty());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("settings.json")).unwrap(),
            content
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
#[test]
fn coercion_error_preserves_raw_committed_hotkeys() {
    let dir = tempfile::tempdir().unwrap();
    let raw = serde_json::json!({"transparency": {"toString": 0}, "hotkeys": {"toggleVisibility":" Cmd + F9 "}});
    std::fs::write(dir.path().join("settings.json"), raw.to_string()).unwrap();
    let probe = Arc::new(SettingsProbe::default());
    let storage = storage(dir.path(), probe.clone());
    let outcome = settings::load_settings(&storage, "Linux");
    assert_eq!(outcome.origin, settings::SettingsOrigin::CoercionErr);
    assert_eq!(outcome.settings, renderer_defaults("Linux"));
    assert_eq!(outcome.committed.toggle_visibility, "CommandOrControl+F9");
    assert!(outcome.coercion_error.is_some());
    assert!(!outcome.save_blocked);
    assert!(outcome.persisted.is_none());
    assert!(probe.writes.lock().unwrap().is_empty());
    assert_eq!(disk(dir.path()), raw);
    assert!(
        storage
            .save(
                FileKind::Settings,
                &serde_json::to_value(outcome.settings).unwrap()
            )
            .is_ok()
    );
}
#[test]
fn recovered_settings_backup_null_then_renderer_defaults_and_failed_recovery_block() {
    for fail in [None, Some(Stage::Backup), Some(Stage::Replace)] {
        let dir = tempfile::tempdir().unwrap();
        let broken = "{not-json";
        std::fs::write(dir.path().join("settings.json"), broken).unwrap();
        let probe = Arc::new(SettingsProbe::default());
        *probe.fail.lock().unwrap() = fail;
        let storage = storage(dir.path(), probe.clone());
        let outcome = settings::load_settings(&storage, "Linux");
        if fail.is_none() {
            assert_eq!(outcome.origin, settings::SettingsOrigin::Recovered);
            assert_eq!(
                &*probe.writes.lock().unwrap(),
                &[
                    Value::Null,
                    serde_json::to_value(renderer_defaults("Linux")).unwrap()
                ]
            );
            let backup = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| p.to_string_lossy().contains(".corrupt-"))
                .unwrap();
            assert_eq!(std::fs::read_to_string(backup).unwrap(), broken);
        } else {
            assert_eq!(outcome.origin, settings::SettingsOrigin::LoadErr);
            assert!(outcome.save_blocked);
            *probe.fail.lock().unwrap() = None;
            assert!(storage.save(FileKind::Settings, &Value::Null).is_err());
            assert_eq!(
                std::fs::read_to_string(dir.path().join("settings.json")).unwrap(),
                broken
            );
        }
    }
}
#[test]
fn legacy_migrates_only_changed_fields_and_migration_failure_is_nonfatal() {
    let dir = tempfile::tempdir().unwrap();
    let probe = Arc::new(SettingsProbe::default());
    let storage = storage(dir.path(), probe.clone());
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"backgroundColor":"dark","hotkeys":{"toggleVisibility":"Alt+F7"}}"#,
    )
    .unwrap();
    let first = settings::load_settings(&storage, "Linux");
    assert_eq!(first.settings.theme, "dark");
    assert_eq!(first.persisted, Some(Ok(())));
    let mut raw = disk(dir.path());
    raw["ignoredExtra"] = true.into();
    std::fs::write(dir.path().join("settings.json"), raw.to_string()).unwrap();
    assert!(
        settings::load_settings(&storage, "Linux")
            .persisted
            .is_none()
    );
    assert_eq!(probe.writes.lock().unwrap().len(), 1);
    assert_eq!(disk(dir.path()), raw);
    std::fs::write(dir.path().join("settings.json"), "null").unwrap();
    *probe.fail.lock().unwrap() = Some(Stage::Replace);
    let failed = settings::load_settings(&storage, "Linux");
    assert!(!failed.save_blocked);
    assert_eq!(failed.origin, settings::SettingsOrigin::ExplicitNull);
    assert!(failed.persisted.unwrap().is_err());
    assert_eq!(failed.settings, renderer_defaults("Linux"));
    assert_eq!(disk(dir.path()), Value::Null);
}
#[gpui::test]
fn pending_changed_save_registers_then_close_registers_again(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, ports, vc) = open_probed(cx, dir.path(), Arc::default());
    click(vc, "settings");
    record(&scene, vc, HotkeyAction::ToggleVisibility, "F8");
    advance(vc, 200);
    ports.events.borrow_mut().clear();
    scene.update_in(vc, |s, w, cx| {
        s.activate(Command::Accept, w, cx);
        assert!(s.settings.saving);
        s.settings_command(
            SettingsCommand::Record(HotkeyAction::ToggleInteractive),
            w,
            cx,
        );
        assert!(s.settings.recorder.is_recording());
    });
    vc.run_until_parked();
    assert_eq!(
        &*ports.events.borrow(),
        &["suspend", "register", "register"]
    );
    scene.read_with(vc, |s, _| {
        assert!(s.dialog.is_none());
        assert!(!s.settings.recorder.is_recording());
        assert_eq!(s.settings.committed.toggle_visibility, "F8");
    });
    assert_eq!(disk(dir.path())["hotkeys"]["toggleVisibility"], "F8");
}
#[gpui::test]
fn stale_stop_then_save_persists_placeholder_literal(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, vc) = open_probed(cx, dir.path(), Arc::default());
    click(vc, "settings");
    record(&scene, vc, HotkeyAction::ToggleVisibility, "F8");
    scene.update_in(vc, |s, w, cx| {
        s.settings_command(
            SettingsCommand::Record(HotkeyAction::ToggleInteractive),
            w,
            cx,
        )
    });
    advance(vc, 199);
    assert!(scene.read_with(vc, |s, _| s.settings.recorder.is_recording()));
    advance(vc, 1);
    assert!(!scene.read_with(vc, |s, _| s.settings.recorder.is_recording()));
    click(vc, "accept");
    assert_eq!(
        disk(dir.path())["hotkeys"]["toggleInteractive"],
        "Press keys now..."
    );
}
#[gpui::test]
fn save_stops_with_restore_cancel_keeps_memory_and_unchanged_save_does_not_retry(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, ports, vc) = open_probed(cx, dir.path(), Arc::default());
    click(vc, "settings");
    record(&scene, vc, HotkeyAction::ToggleVisibility, "F8");
    click(vc, "cancel");
    assert_eq!(
        scene.read_with(vc, |s, _| s.settings.live.hotkeys.toggle_visibility.clone()),
        "F8"
    );
    assert_ne!(disk(dir.path())["hotkeys"]["toggleVisibility"], "F8");
    click(vc, "settings");
    assert_eq!(
        scene.read_with(vc, |s, _| s.settings.displayed.toggle_visibility.clone()),
        "F8"
    );
    click(vc, "accept");
    click(vc, "settings");
    ports.events.borrow_mut().clear();
    click(vc, "accept");
    assert!(ports.events.borrow().is_empty());
    click(vc, "settings");
    record(&scene, vc, HotkeyAction::ToggleVisibility, "F9");
    click(vc, "accept");
    assert_eq!(
        disk(dir.path())["hotkeys"]["toggleVisibility"],
        "F8",
        "Save restores pre-record display before reading controls"
    );
    assert_eq!(&*ports.events.borrow(), &["suspend", "register"]);
}
#[gpui::test]
fn settings_fifo_immutable_payloads_and_failure_retains_controls(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let probe = Arc::new(SettingsProbe::default());
    let (scene, _, vc) = open_probed(cx, dir.path(), probe.clone());
    probe.writes.lock().unwrap().clear();
    click(vc, "settings");
    scene.update_in(vc, |s, w, cx| {
        for volume in [11, 22, 33] {
            s.settings_command(SettingsCommand::Volume(volume), w, cx);
            s.activate(Command::Accept, w, cx);
        }
        assert_eq!(s.settings.pending.len(), 2);
    });
    vc.run_until_parked();
    assert_eq!(
        probe
            .writes
            .lock()
            .unwrap()
            .iter()
            .map(|v| v["sounds"]["volume"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [11, 22, 33]
    );
    assert_eq!(disk(dir.path())["sounds"]["volume"], 33);
    click(vc, "settings");
    *probe.fail.lock().unwrap() = Some(Stage::Replace);
    scene.update_in(vc, |s, w, cx| {
        s.settings_command(SettingsCommand::Volume(77), w, cx)
    });
    click(vc, "accept");
    scene.read_with(vc, |s, _| {
        assert!(matches!(s.dialog, Some(Dialog::Settings)));
        assert_eq!(s.settings.live.sounds.volume, 77);
        assert!(
            s.settings
                .notice
                .as_ref()
                .unwrap()
                .starts_with("Error saving settings:")
        );
        assert!(s.error.is_none());
    });
    assert_eq!(disk(dir.path())["sounds"]["volume"], 33);
    let notice = vc.debug_bounds("settings-notice").unwrap();
    let footer = vc.debug_bounds("accept").unwrap();
    assert!(
        notice.top() >= gpui::px(0.) && notice.bottom() <= footer.top(),
        "save error must be visible, not below the scrolled body"
    );
    *probe.fail.lock().unwrap() = None;
    click(vc, "accept");
    assert_eq!(disk(dir.path())["sounds"]["volume"], 77);
}
#[gpui::test]
fn reset_defaults_is_preview_until_save_and_decode_failure_keeps_recording(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, ports, vc) = open_probed(cx, dir.path(), Arc::default());
    click(vc, "settings");
    record(&scene, vc, HotkeyAction::ToggleVisibility, "F8");
    advance(vc, 200);
    click(vc, "accept");
    click(vc, "settings");
    ports.events.borrow_mut().clear();
    scene.update_in(vc, |s, w, cx| {
        s.settings_command(SettingsCommand::Reset, w, cx)
    });
    vc.run_until_parked();
    click(vc, "accept");
    assert_eq!(
        scene.read_with(vc, |s, _| s.settings.live.hotkeys.clone()),
        renderer_defaults(settings::platform()).hotkeys
    );
    assert_eq!(disk(dir.path())["hotkeys"]["toggleVisibility"], "F8");
    assert!(ports.events.borrow().is_empty());
    scene.update_in(vc, |s, w, cx| {
        s.settings_command(
            SettingsCommand::Record(HotkeyAction::ToggleVisibility),
            w,
            cx,
        );
        for error in [
            RecorderDecodeError::SnapshotFailed("test".into()),
            RecorderDecodeError::UnrepresentableKey("test".into()),
            RecorderDecodeError::UnsupportedTranslation("test".into()),
        ] {
            assert!(s.recorder_input(Err(error), w, cx));
            assert!(s.settings.recorder.is_recording());
        }
        assert!(s.recorder_input(
            Ok(KeyEvent {
                key: "Escape".into(),
                ..Default::default()
            }),
            w,
            cx
        ));
        assert!(
            s.settings.recorder.is_recording(),
            "bare Escape consumes without cancelling"
        );
        assert!(!s.editor.focus_handle(cx).is_focused(w));
        assert!(!s.input.focus_handle(cx).is_focused(w));
    });
    vc.run_until_parked();
    vc.simulate_keystrokes("tab escape a");
    assert!(scene.read_with(vc, |s, _| s.settings.recorder.is_recording()));
    click(vc, "cancel");
    assert!(!scene.read_with(vc, |s, _| s.settings.recorder.is_recording()));
}

#[gpui::test]
fn tray_quit_waits_all_write_queues(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let probe = Arc::new(SettingsProbe::default());
    let (scene, ports, vc) = open_probed(cx, dir.path(), probe.clone());
    probe.writes.lock().unwrap().clear();
    scene.update_in(vc, |s, w, cx| {
        s.activate(Command::Settings(SettingsCommand::Open), w, cx);
        for volume in [17, 29] {
            s.settings_command(SettingsCommand::Volume(volume), w, cx);
            s.activate(Command::Accept, w, cx);
        }
        s.close_dialog(w, cx);
        s.activate(Command::Template(TemplateCommand::Save), w, cx);
        s.editor
            .update(cx, |e, cx| e.set_text("quit template", w, cx));
        s.activate(Command::Accept, w, cx);
        s.close_dialog(w, cx);
        s.dispatch_background(
            BackgroundCommand::AddTask {
                text: "before exit".into(),
                parent: None,
            },
            cx,
        );
        s.dispatch_background(BackgroundCommand::Save, cx);
        assert!(s.saving && s.templates.saving && s.settings.saving);
        assert!(!s.pending.is_empty() && !s.settings.pending.is_empty());
        s.tray_command(TrayCommand::Exit, w, cx);
        assert!(!s.overlay.shutdown);
        assert!(!s.quit_ready());
    });
    assert!(!ports.events.borrow().iter().any(|e| e == "shutdown"));
    vc.run_until_parked();
    advance(vc, 10);
    assert!(scene.read_with(vc, |s, _| s.overlay.shutdown && s.quit_ready()));
    assert_eq!(
        ports.events.borrow().last().map(String::as_str),
        Some("shutdown")
    );
    assert_eq!(
        probe
            .writes
            .lock()
            .unwrap()
            .iter()
            .map(|v| v["sounds"]["volume"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [17, 29]
    );
    assert_eq!(disk(dir.path())["sounds"]["volume"], 29);
    let tasks: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("current_tasks.json")).unwrap())
            .unwrap();
    assert!(
        tasks["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["text"] == "before exit")
    );
    let templates: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("templates.json")).unwrap()).unwrap();
    assert!(
        templates
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "quit template")
    );
}
#[gpui::test]
fn quit_before_load_waits_migration_and_other_loads(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let ports = Rc::new(FakeOverlay::default());
    cx.update(crate::components::text_input::init);
    let (scene, vc) = cx.add_window_view(|w, cx| {
        let mut s = TaskApp::new_with_overlay(
            dir.path().into(),
            OverlayConfig {
                ports: ports.clone(),
                register_delay: Duration::from_millis(2000),
            },
            w,
            cx,
        );
        s.tray_command(TrayCommand::Exit, w, cx);
        assert!(s.settings.loading && s.task_loading && !s.templates_load_settled());
        assert!(!s.overlay.shutdown);
        s
    });
    vc.run_until_parked();
    assert!(dir.path().join("settings.json").exists());
    assert!(!dir.path().join("templates.json").exists());
    assert!(!dir.path().join("current_tasks.json").exists());
    advance(vc, 10);
    assert!(scene.read_with(vc, |s, _| s.overlay.shutdown));
    advance(vc, 2000);
    assert_eq!(&*ports.events.borrow(), &["shutdown"]);
}
#[gpui::test]
fn startup_registration_waits_two_seconds_and_raw_main_mapping(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"hotkeys":{"toggleVisibility":"Ctrl + F7"}}"#,
    )
    .unwrap();
    let (scene, ports, vc) = overlay_tests::open(cx, dir.path(), Duration::from_millis(2000));
    assert!(ports.events.borrow().is_empty());
    assert!(!scene.read_with(vc, |s, _| s.settings.loading));
    advance(vc, 1999);
    assert!(ports.events.borrow().is_empty());
    advance(vc, 1);
    assert_eq!(&*ports.events.borrow(), &["register"]);
    assert_eq!(
        ports.mappings.borrow()[0].toggle_visibility,
        "CommandOrControl+F7"
    );
    assert_eq!(disk(dir.path())["hotkeys"]["toggleVisibility"], "Ctrl + F7");
}
#[gpui::test]
fn settings_load_failure_does_not_block_task_or_template_writes(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("settings.json"), "undefined").unwrap();
    let (scene, _, vc) = open_probed(cx, dir.path(), Arc::default());
    assert!(scene.read_with(vc, |s, _| s.writable() && s.templates_ready()));
    click(vc, "settings");
    click(vc, "accept");
    assert!(scene.read_with(vc, |s, _| matches!(s.dialog, Some(Dialog::Settings))));
    click(vc, "cancel");
    scene.update_in(vc, |s, w, cx| w.focus(&s.input.focus_handle(cx), cx));
    vc.simulate_keystrokes("a enter");
    vc.run_until_parked();
    click(vc, "template-save");
    vc.simulate_keystrokes("b enter");
    vc.run_until_parked();
    assert!(dir.path().join("current_tasks.json").exists());
    assert!(dir.path().join("templates.json").exists());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("settings.json")).unwrap(),
        "undefined"
    );
    assert!(scene.read_with(vc, |s, _| s.quit_ready()));
}
#[gpui::test]
fn recorder_ignored_rejected_duplicate_and_switch_effects_keep_order(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, ports, vc) = open_probed(cx, dir.path(), Arc::default());
    click(vc, "settings");
    ports.events.borrow_mut().clear();
    scene.update_in(vc, |s, w, cx| {
        let committed = s.settings.committed.clone();
        let displayed = s.settings.displayed.clone();
        let live_hotkeys = s.settings.live.hotkeys.clone();
        s.settings_command(
            SettingsCommand::Record(HotkeyAction::ToggleVisibility),
            w,
            cx,
        );
        for key in ["Shift", "a"] {
            assert!(s.recorder_input(
                Ok(KeyEvent {
                    key: key.into(),
                    ..Default::default()
                }),
                w,
                cx
            ));
            assert_eq!(s.settings.displayed.toggle_visibility, "Press keys now...");
            assert!(s.settings.recorder.is_recording());
        }
        assert!(
            s.settings.notice.is_none(),
            "bare keys are silently ignored"
        );
        s.recorder_input(
            Ok(KeyEvent {
                key: "A".into(),
                shift_key: true,
                ..Default::default()
            }),
            w,
            cx,
        );
        assert!(s.settings.notice.is_some(), "shift-only key must alert");
        s.recorder_input(
            Ok(KeyEvent {
                key: "I".into(),
                ctrl_key: true,
                shift_key: true,
                ..Default::default()
            }),
            w,
            cx,
        );
        assert!(
            s.settings
                .notice
                .as_ref()
                .unwrap()
                .contains("already assigned")
        );
        assert_eq!(
            s.settings.live.hotkeys.toggle_visibility,
            live_hotkeys.toggle_visibility
        );
        s.settings_command(
            SettingsCommand::Record(HotkeyAction::ToggleInteractive),
            w,
            cx,
        );
        assert_eq!(
            s.settings.displayed.toggle_visibility,
            displayed.toggle_visibility
        );
        assert_eq!(s.settings.displayed.toggle_interactive, "Press keys now...");
        s.settings_command(
            SettingsCommand::Record(HotkeyAction::ToggleInteractive),
            w,
            cx,
        );
        assert_eq!(
            s.settings.displayed.toggle_interactive,
            displayed.toggle_interactive
        );
        assert!(!s.settings.recorder.is_recording());
        assert!(!s.recorder_input(
            Ok(KeyEvent {
                key: "F9".into(),
                ..Default::default()
            }),
            w,
            cx
        ));
        assert!(
            ports
                .mappings
                .borrow()
                .iter()
                .all(|keys| keys == &committed)
        );
    });
    assert_eq!(
        &*ports.events.borrow(),
        &["suspend", "register", "suspend", "register"]
    );
    advance(vc, 200);
    assert_eq!(
        ports.events.borrow().len(),
        4,
        "ignored/rejected keys queue no deferred stop"
    );
}
#[gpui::test]
fn settings_controls_scroll_footer_ranges_and_reset_use_dispatched_input(cx: &mut TestAppContext) {
    for scale in [1., 1.5, 2.] {
        let dir = tempfile::tempdir().unwrap();
        let (scene, _, vc) = open_probed(cx, dir.path(), Arc::default());
        vc.simulate_scale_factor_change(scale);
        vc.simulate_resize(gpui::size(gpui::px(450.), gpui::px(600.)));
        click(vc, "settings");
        let footer = vc.debug_bounds("accept").unwrap();
        assert!(footer.top() >= gpui::px(0.) && footer.bottom() <= gpui::px(600.));
        click(vc, "transparency");
        vc.simulate_keystrokes("home right right");
        assert_eq!(scene.read_with(vc, |s, _| s.settings.live.transparency), 2);
        click(vc, "theme-sounds");
        assert!(scene.read_with(vc, |s, _| s.settings.live.sounds.enabled));
        let row = vc.debug_bounds("row-0").unwrap();
        let body = vc.debug_bounds("dialog-body").unwrap();
        vc.simulate_event(gpui::ScrollEvent {
            position: body.center(),
            delta: gpui::ScrollDelta::Lines(gpui::point(0., -1000.)),
            ..Default::default()
        });
        vc.run_until_parked();
        assert_eq!(vc.debug_bounds("accept").unwrap(), footer);
        assert_eq!(vc.debug_bounds("row-0").unwrap(), row);
        click(vc, "reset-hotkeys");
        assert!(scene.read_with(vc, |s, _| matches!(s.dialog, Some(Dialog::SettingsReset))));
        vc.simulate_keystrokes("tab enter");
        assert!(scene.read_with(vc, |s, _| matches!(s.dialog, Some(Dialog::SettingsReset))));
        click(vc, "accept");
        click(vc, "accept");
        assert_eq!(disk(dir.path())["transparency"], 2);
        assert_eq!(disk(dir.path())["sounds"]["enabled"], true);
    }
}
