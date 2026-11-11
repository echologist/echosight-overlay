use super::*;
use gpui::EntityInputHandler;
use std::sync::atomic::{AtomicUsize, Ordering};

fn assert_visible_notice(vc: &mut VisualTestContext, modal: bool, height: f32) {
    let notice = vc
        .debug_bounds("settings-notice")
        .expect("rendered settings failure");
    assert!(notice.size.width > gpui::px(0.) && notice.size.height > gpui::px(0.));
    assert!(notice.top() >= gpui::px(0.) && notice.bottom() <= gpui::px(height));
    assert!(notice.size.height <= gpui::px(height * 0.2));
    if modal {
        // The error must occupy the opaque panel's fixed notice area, not the
        // underlying main status or the potentially scrolled-away form body.
        let panel = vc.debug_bounds("dialog-panel").unwrap();
        let body = vc.debug_bounds("dialog-body").unwrap();
        let accept = vc.debug_bounds("accept").unwrap();
        let cancel = vc.debug_bounds("cancel").unwrap();
        assert!(notice.left() >= panel.left() && notice.right() <= panel.right());
        assert!(notice.top() >= body.bottom() && notice.bottom() <= accept.top());
        assert!(panel.top() >= gpui::px(0.) && panel.bottom() <= gpui::px(height));
        assert!(accept.bottom() <= panel.bottom() && cancel.bottom() <= panel.bottom());
    }
}

fn assert_resized_notice(vc: &mut VisualTestContext, modal: bool) {
    for scale in [1., 1.5, 2.] {
        vc.simulate_scale_factor_change(scale);
        for (width, height) in [(450., 600.), (500., 700.)] {
            vc.simulate_resize(gpui::size(gpui::px(width), gpui::px(height)));
            vc.run_until_parked();
            assert_visible_notice(vc, modal, height);
        }
    }
}

#[derive(Default)]
struct FailSecondSettingsWrite {
    replaces: AtomicUsize,
    snapshots: Mutex<Vec<Value>>,
}
impl StorageHooks for FailSecondSettingsWrite {
    fn before(&self, stage: Stage, path: &std::path::Path) -> std::io::Result<()> {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if stage == Stage::Sync && name.starts_with(".settings.json.") {
            self.snapshots
                .lock()
                .unwrap()
                .push(serde_json::from_slice(&std::fs::read(path)?).unwrap());
        }
        if stage == Stage::Replace
            && name == "settings.json"
            && self.replaces.fetch_add(1, Ordering::SeqCst) == 1
        {
            return Err(std::io::Error::other(
                "review second-settings-write failure",
            ));
        }
        Ok(())
    }
}
fn open_fifo<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
) -> (
    Entity<TaskApp>,
    Rc<FakeOverlay>,
    Arc<FailSecondSettingsWrite>,
    &'a mut VisualTestContext,
) {
    std::fs::write(
        dir.join("settings.json"),
        serde_json::to_vec(&renderer_defaults(settings::platform())).unwrap(),
    )
    .unwrap();
    cx.update(crate::components::text_input::init);
    let probe = Arc::new(FailSecondSettingsWrite::default());
    let ports = Rc::new(FakeOverlay::default());
    ports.visible.set(true);
    let (scene, vc) = cx.add_window_view(|w, cx| {
        let paths = DataPaths::new(dir).unwrap();
        let mut app = TaskApp::with_storage(
            Ok((Storage::with_hooks(paths.clone(), probe.clone()), paths)),
            w,
            cx,
        );
        app.overlay.ports = Some(ports.clone());
        app.overlay.delay_elapsed = true;
        app
    });
    vc.simulate_resize(gpui::size(gpui::px(500.), gpui::px(700.)));
    vc.run_until_parked();
    (scene, ports, probe, vc)
}

#[gpui::test]
fn review_fifo_continues_after_failed_middle_write_with_immutable_hotkeys(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, ports, probe, vc) = open_fifo(cx, dir.path());
    click(vc, "settings");
    ports.events.borrow_mut().clear();
    ports.mappings.borrow_mut().clear();
    scene.update_in(vc, |s, w, cx| {
        for (volume, shortcut) in [(11, "F7"), (22, "F8"), (33, "F9")] {
            s.settings_command(SettingsCommand::Volume(volume), w, cx);
            s.settings.displayed.toggle_visibility = shortcut.into();
            s.activate(Command::Accept, w, cx);
        }
    });
    vc.run_until_parked();
    assert_eq!(probe.replaces.load(Ordering::SeqCst), 3);
    assert_eq!(
        probe
            .snapshots
            .lock()
            .unwrap()
            .iter()
            .map(|v| v["sounds"]["volume"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [11, 22, 33]
    );
    assert_eq!(
        ports
            .mappings
            .borrow()
            .iter()
            .map(|h| h.toggle_visibility.clone())
            .collect::<Vec<_>>(),
        ["F7", "F9"]
    );
    assert_eq!(disk(dir.path())["sounds"]["volume"], 33);
    scene.read_with(vc, |s, _| {
        assert!(!s.settings.saving && s.settings.pending.is_empty());
        assert_eq!(s.settings.committed.toggle_visibility, "F9");
    });
}

#[gpui::test]
fn review_late_fifo_failure_is_visible_after_earlier_success_closes_modal(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, probe, vc) = open_fifo(cx, dir.path());
    click(vc, "settings");
    scene.update_in(vc, |s, w, cx| {
        for volume in [11, 22] {
            s.settings_command(SettingsCommand::Volume(volume), w, cx);
            s.activate(Command::Accept, w, cx);
        }
    });
    vc.run_until_parked();
    assert_eq!(probe.replaces.load(Ordering::SeqCst), 2);
    assert_eq!(disk(dir.path())["sounds"]["volume"], 11);
    let status = scene.read_with(vc, |s, _| {
        assert!(s.dialog.is_none());
        assert_eq!(s.settings.live.sounds.volume, 22);
        assert!(
            s.settings
                .notice
                .as_deref()
                .unwrap()
                .contains("Error saving settings:")
        );
        println!(
            "final status={:?}; hidden notice={:?}; disk volume=11; live volume=22",
            s.status, s.settings.notice
        );
        s.status.clone()
    });
    let visible_notice = vc.debug_bounds("settings-notice");
    println!("rendered settings-notice bounds={visible_notice:?}");
    assert!(
        visible_notice.is_some() || status.contains("Error saving settings:"),
        "late FIFO failure is invisible and the main view still claims settings saved successfully"
    );
    assert!(status.starts_with("Error saving settings:"));
    assert_resized_notice(vc, false);
    // Retrying uses the retained live controls and clears the failure on success.
    click(vc, "settings");
    assert_visible_notice(vc, true, 700.);
    click(vc, "accept");
    assert_eq!(disk(dir.path())["sounds"]["volume"], 22);
    assert!(vc.debug_bounds("settings-notice").is_none());
    assert!(scene.read_with(vc, |s, _| {
        s.status.starts_with("Settings saved successfully!")
    }));
}

#[gpui::test]
fn review_save_failure_after_cancel_remains_visible(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let probe = Arc::new(SettingsProbe::default());
    let (scene, _, vc) = open_probed(cx, dir.path(), probe.clone());
    click(vc, "settings");
    *probe.fail.lock().unwrap() = Some(Stage::Replace);
    scene.update_in(vc, |s, w, cx| {
        s.settings_command(SettingsCommand::Volume(77), w, cx);
        s.activate(Command::Accept, w, cx);
        assert!(s.settings.saving);
        s.activate(Command::Cancel, w, cx);
    });
    vc.run_until_parked();
    assert_eq!(disk(dir.path())["sounds"]["volume"], 60);
    let status = scene.read_with(vc, |s, _| {
        assert!(s.dialog.is_none());
        assert!(
            s.settings
                .notice
                .as_deref()
                .unwrap()
                .contains("Error saving settings:")
        );
        println!(
            "after Cancel status={:?}; hidden notice={:?}",
            s.status, s.settings.notice
        );
        s.status.clone()
    });
    assert!(
        vc.debug_bounds("settings-notice").is_some() || status.contains("Error saving settings:"),
        "a pending settings save failure must be surfaced even after Cancel"
    );
    assert_resized_notice(vc, false);
}

#[gpui::test]
fn review_reset_during_recording_preserves_restore_then_save_race(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, vc) = open_probed(cx, dir.path(), Arc::default());
    click(vc, "settings");
    record(&scene, vc, HotkeyAction::ToggleVisibility, "F8");
    advance(vc, 200);
    click(vc, "accept");
    click(vc, "settings");
    scene.update_in(vc, |s, w, cx| {
        s.settings_command(
            SettingsCommand::Record(HotkeyAction::ToggleVisibility),
            w,
            cx,
        );
        s.settings_command(SettingsCommand::Reset, w, cx);
        s.activate(Command::Accept, w, cx);
        assert!(s.settings.recorder.is_recording());
        assert_eq!(
            s.settings.live.hotkeys,
            renderer_defaults(settings::platform()).hotkeys
        );
        s.activate(Command::Accept, w, cx);
    });
    vc.run_until_parked();
    assert_eq!(
        disk(dir.path())["hotkeys"]["toggleVisibility"],
        "F8",
        "stop restores pre-reset display before Save reads controls"
    );
}

fn failure_over_newer_draft(cx: &mut TestAppContext, template: bool) {
    for queued_success in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (scene, ports, probe, vc) = open_fifo(cx, dir.path());
        let original_disk = disk(dir.path());
        let original_committed = scene.read_with(vc, |s, _| s.settings.committed.clone());
        if !queued_success {
            probe.replaces.store(1, Ordering::SeqCst);
        }
        ports.mappings.borrow_mut().clear();
        click(vc, "settings");
        let (editor, focus, task_id) = scene.update_in(vc, |s, w, cx| {
            if queued_success {
                s.settings_command(SettingsCommand::Volume(11), w, cx);
                s.settings.displayed.toggle_visibility = "F7".into();
                s.activate(Command::Accept, w, cx);
            }
            s.settings_command(SettingsCommand::Volume(22), w, cx);
            s.settings.displayed.toggle_visibility = "F8".into();
            s.activate(Command::Accept, w, cx);
            assert!(s.settings.saving);
            s.activate(Command::Cancel, w, cx);
            let task_id = s.state().unwrap().tasks()[0].id;
            s.activate(
                if template {
                    Command::Template(TemplateCommand::Save)
                } else {
                    Command::Edit(task_id)
                },
                w,
                cx,
            );
            s.select_editor = false;
            s.editor.update(cx, |e, cx| {
                e.set_text("untouched draft", w, cx);
                e.set_selected_text_range(2..7, w, cx);
            });
            s.high_priority = true;
            s.templates.selected = Some(0);
            w.focus(&s.editor.focus_handle(cx), cx);
            (s.editor.entity_id(), w.focused(cx), task_id)
        });
        vc.run_until_parked();
        scene.update_in(vc, |s, w, cx| {
            if template {
                assert!(matches!(
                    s.dialog,
                    Some(Dialog::Template(TemplateDialog::Save))
                ));
            } else {
                assert!(matches!(s.dialog, Some(Dialog::Edit(id, _)) if id == task_id));
            }
            assert_eq!(s.editor.entity_id(), editor);
            assert_eq!(s.editor.read(cx).text(), "untouched draft");
            assert_eq!(
                s.editor.update(cx, |e, cx| e
                    .selected_text_range(false, w, cx)
                    .unwrap()
                    .range),
                2..7
            );
            assert_eq!(w.focused(cx), focus);
            assert!(s.high_priority);
            assert_eq!(s.templates.selected, Some(0));
            assert_eq!(s.settings.live.sounds.volume, 22);
            assert_eq!(s.settings.live.hotkeys.toggle_visibility, "F8");
            assert_eq!(s.settings.displayed.toggle_visibility, "F8");
            assert!(!s.settings.saving && s.settings.pending.is_empty());
            if queued_success {
                assert_eq!(s.settings.committed.toggle_visibility, "F7");
            } else {
                assert_eq!(s.settings.committed, original_committed);
            }
        });
        if queued_success {
            assert_eq!(disk(dir.path())["sounds"]["volume"], 11);
            assert_eq!(disk(dir.path())["hotkeys"]["toggleVisibility"], "F7");
            assert_eq!(ports.mappings.borrow().len(), 1);
            assert_eq!(ports.mappings.borrow()[0].toggle_visibility, "F7");
        } else {
            assert_eq!(disk(dir.path()), original_disk);
            assert!(ports.mappings.borrow().is_empty());
        }
        assert_resized_notice(vc, true);
        // The notice is neither a replacement modal nor a focus-taking alert.
        vc.simulate_keystrokes("x");
        assert_eq!(
            scene.read_with(vc, |s, cx| s.editor.read(cx).text().to_owned()),
            "unxed draft"
        );
        click(vc, "cancel");
        assert_visible_notice(vc, false, 700.);
        click(vc, "settings");
        click(vc, "accept");
        assert_eq!(disk(dir.path())["sounds"]["volume"], 22);
        assert_eq!(disk(dir.path())["hotkeys"]["toggleVisibility"], "F8");
        assert!(vc.debug_bounds("settings-notice").is_none());
    }
}

#[gpui::test]
fn late_failure_is_visible_inside_newer_task_dialog_without_changing_draft(
    cx: &mut TestAppContext,
) {
    failure_over_newer_draft(cx, false);
}

#[gpui::test]
fn late_failure_is_visible_inside_newer_template_dialog_without_changing_draft(
    cx: &mut TestAppContext,
) {
    failure_over_newer_draft(cx, true);
}
