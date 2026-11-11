use super::*;
use echosight_core::settings::{HotkeyAction, HotkeySettings};
use gpui::{EntityInputHandler, TestAppContext, VisualTestContext};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

#[derive(Default)]
pub(super) struct FakeOverlay {
    pub events: RefCell<Vec<String>>,
    pub mappings: RefCell<Vec<HotkeySettings>>,
    pub visible: Cell<bool>,
    pub summary: RefCell<Option<RegisterSummary>>,
    pub release: RefCell<ReleaseSummary>,
}
impl OverlayPorts for FakeOverlay {
    fn register_hotkeys(&self, keys: &HotkeySettings) -> RegisterSummary {
        self.events.borrow_mut().push("register".into());
        self.mappings.borrow_mut().push(keys.clone());
        self.summary
            .borrow()
            .clone()
            .unwrap_or_else(|| RegisterSummary {
                actions: HotkeyAction::ALL
                    .map(|a| (a, ActionStatus::Inactive("injected conflict".into()))),
                retained: vec![],
            })
    }
    fn suspend_hotkeys(&self) -> ReleaseSummary {
        self.events.borrow_mut().push("suspend".into());
        self.release.borrow().clone()
    }
    fn shutdown(&self) -> ReleaseSummary {
        self.events.borrow_mut().push("shutdown".into());
        ReleaseSummary::default()
    }
    fn is_visible(&self) -> Option<bool> {
        Some(self.visible.get())
    }
    fn show(&self) -> Result<(), String> {
        self.visible.set(true);
        self.events.borrow_mut().push("show/topmost".into());
        Ok(())
    }
    fn hide(&self) -> Result<(), String> {
        self.visible.set(false);
        self.events.borrow_mut().push("hide".into());
        Ok(())
    }
    fn focus(&self) -> Result<(), String> {
        self.events.borrow_mut().push("focus".into());
        Ok(())
    }
    fn set_click_through(&self, enabled: bool) -> Result<(), String> {
        self.events
            .borrow_mut()
            .push(format!("click-through:{enabled}"));
        Ok(())
    }
}
pub(super) fn open<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
    delay: Duration,
) -> (Entity<TaskApp>, Rc<FakeOverlay>, &'a mut VisualTestContext) {
    cx.update(crate::components::text_input::init);
    let ports = Rc::new(FakeOverlay::default());
    ports.visible.set(true);
    let config = OverlayConfig {
        ports: ports.clone(),
        register_delay: delay,
    };
    let (scene, vc) =
        cx.add_window_view(|w, cx| TaskApp::new_with_overlay(dir.into(), config, w, cx));
    vc.simulate_resize(gpui::size(gpui::px(500.), gpui::px(700.)));
    vc.run_until_parked();
    (scene, ports, vc)
}
#[gpui::test]
fn global_actions_under_modals_preserve_drafts(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, vc) = open(cx, dir.path(), Duration::ZERO);
    let (id, template) = scene.read_with(vc, |s, _| {
        (
            s.state().unwrap().tasks()[0].id,
            s.templates.collection.as_ref().unwrap()[0].clone(),
        )
    });
    let dialogs = vec![
        Dialog::Edit(id, false),
        Dialog::Child(id),
        Dialog::Delete(id),
        Dialog::Triggers(id),
        Dialog::Settings,
        Dialog::SettingsReset,
        Dialog::Quit,
        Dialog::Template(TemplateDialog::Choose),
        Dialog::Template(TemplateDialog::Save),
        Dialog::Template(TemplateDialog::Load(template.clone())),
        Dialog::Template(TemplateDialog::Delete(template)),
        Dialog::Template(TemplateDialog::Import),
        Dialog::Template(TemplateDialog::Community),
        Dialog::Template(TemplateDialog::Replace(templates::Incoming::Community(0))),
        Dialog::Template(TemplateDialog::Preview("preview draft".into())),
    ];
    for dialog in dialogs {
        scene.update_in(vc, |s, w, cx| {
            s.dialog = Some(dialog.clone());
            s.select_editor = false;
            s.editor.update(cx, |e, cx| {
                e.set_text("untouched draft", w, cx);
                e.set_selected_text_range(2..7, w, cx);
            });
            s.high_priority = true;
            s.templates.selected = Some(0);
            w.focus(&s.editor.focus_handle(cx), cx);
        });
        vc.run_until_parked();
        let (editor, focus) = scene.update_in(vc, |s, w, cx| {
            let editor = s.editor.entity_id();
            let focus = w.focused(cx);
            s.global_hotkey(HotkeyAction::CompleteNextTask, w, cx);
            assert!(
                s.state().unwrap().tasks()[0].completed,
                "global action must bypass every modal guard"
            );
            s.global_hotkey(HotkeyAction::UndoLastAction, w, cx);
            assert!(!s.state().unwrap().tasks()[0].completed);
            s.global_hotkey(HotkeyAction::RedoLastAction, w, cx);
            assert!(s.state().unwrap().tasks()[0].completed);
            s.global_hotkey(HotkeyAction::UndoLastAction, w, cx);
            (editor, focus)
        });
        vc.run_until_parked();
        scene.update_in(vc, |s, w, cx| {
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
            assert_eq!(
                std::mem::discriminant(s.dialog.as_ref().unwrap()),
                std::mem::discriminant(&dialog)
            );
            assert!(s.high_priority);
            assert_eq!(s.templates.selected, Some(0));
        });
    }
}
#[gpui::test]
fn local_actions_keep_modal_guard(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, vc) = open(cx, dir.path(), Duration::ZERO);
    scene.update_in(vc, |s, w, cx| {
        let id = s.state().unwrap().tasks()[0].id;
        s.activate(Command::Edit(id), w, cx);
        s.activate(Command::Complete, w, cx);
        assert!(!s.state().unwrap().tasks()[0].completed);
    });
}
#[gpui::test]
fn tray_show_recovers_failed_toggle_registration(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, ports, vc) = open(cx, dir.path(), Duration::ZERO);
    scene.update_in(vc, |s, w, cx| {
        s.tray_command(TrayCommand::Hide, w, cx);
        s.tray_command(TrayCommand::Show, w, cx);
    });
    assert_eq!(
        &*ports.events.borrow(),
        &["register", "hide", "show/topmost", "focus"]
    );
    assert!(ports.visible.get());
}
#[gpui::test]
fn settings_dialog_is_dispatched_and_bounded(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (_scene, _, vc) = open(cx, dir.path(), Duration::ZERO);
    let bounds = vc.debug_bounds("settings").expect("settings menu control");
    vc.simulate_click(bounds.center(), Default::default());
    vc.run_until_parked();
    let footer = vc.debug_bounds("accept").expect("save settings footer");
    assert!(footer.bottom() <= gpui::px(700.));
}

#[gpui::test]
fn edit_restored_target_uses_current_mode(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, vc) = open(cx, dir.path(), Duration::ZERO);
    scene.update_in(vc, |s, w, cx| {
        let id = s.state().unwrap().tasks()[0].id;
        let mut background = s.state().unwrap().tasks().to_vec();
        background[0].mode = TaskMode::Background;
        let mut main = background.clone();
        main[0].mode = TaskMode::Main;
        s.dispatch_background(
            BackgroundCommand::Replace {
                tasks: background,
                current_template: None,
            },
            cx,
        );
        s.dispatch_background(
            BackgroundCommand::CaptureUndo("mode restoration".into()),
            cx,
        );
        s.dispatch_background(
            BackgroundCommand::Replace {
                tasks: main,
                current_template: None,
            },
            cx,
        );
        s.activate(Command::Edit(id), w, cx);
        s.high_priority = true;
        s.global_hotkey(HotkeyAction::UndoLastAction, w, cx);
        assert_eq!(
            s.state().unwrap().find_task(id).unwrap().mode,
            TaskMode::Background
        );
        s.activate(Command::Accept, w, cx);
        assert_eq!(
            s.state()
                .unwrap()
                .find_task(id)
                .unwrap()
                .background_options
                .as_ref()
                .map(|o| o.priority),
            Some(BackgroundPriority::High)
        );
    });
}

#[gpui::test]
fn missing_child_and_trigger_owners_keep_drafts_without_write(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, vc) = open(cx, dir.path(), Duration::ZERO);
    scene.update_in(vc, |s, w, cx| {
        for triggers in [false, true] {
            let id = s
                .state()
                .unwrap()
                .tasks()
                .first()
                .map(|t| t.id)
                .unwrap_or(TaskId(99.));
            s.dialog = Some(if triggers {
                Dialog::Triggers(id)
            } else {
                Dialog::Child(id)
            });
            s.editor
                .update(cx, |e, cx| e.set_text("keep this draft", w, cx));
            s.dispatch_background(
                BackgroundCommand::Replace {
                    tasks: vec![],
                    current_template: None,
                },
                cx,
            );
            let pending = s.pending.len();
            s.activate(Command::Accept, w, cx);
            assert!(
                s.dialog.is_some(),
                "missing child/trigger target retains draft"
            );
            assert_eq!(s.editor.read(cx).text(), "keep this draft");
            assert_eq!(s.pending.len(), pending, "no save for absent owner");
        }
    });
}

#[gpui::test]
fn global_visibility_feedback_and_local_chord_gates(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, ports, vc) = open(cx, dir.path(), Duration::ZERO);
    scene.update_in(vc, |s, w, cx| {
        s.global_hotkey(HotkeyAction::ToggleInteractive, w, cx);
        assert!(!s.overlay.interactive);
        s.global_hotkey(HotkeyAction::ToggleVisibility, w, cx);
        for action in [
            HotkeyAction::ToggleInteractive,
            HotkeyAction::CompleteNextTask,
            HotkeyAction::UndoLastAction,
            HotkeyAction::RedoLastAction,
        ] {
            s.global_hotkey(action, w, cx);
        }
        assert!(!s.overlay.interactive);
        assert!(!s.state().unwrap().tasks()[0].completed);
        s.tray_command(TrayCommand::Show, w, cx);
        assert!(!s.overlay.interactive && !s.overlay.manually_hidden);
        s.global_hotkey(HotkeyAction::CompleteNextTask, w, cx);
        assert!(s.state().unwrap().tasks()[0].completed);
        assert!(
            s.feedback.is_none(),
            "click-through completion has no interactive feedback"
        );
        w.focus(&s.input.focus_handle(cx), cx);
    });
    vc.run_until_parked();
    vc.simulate_keystrokes("a ctrl-shift-z");
    scene.update_in(vc, |s, w, cx| {
        assert_eq!(
            s.input.read(cx).text(),
            "a",
            "ordinary text keys stay local"
        );
        assert!(
            s.state().unwrap().tasks()[0].completed,
            "production ports disable local global-chord fallback"
        );
        s.global_hotkey(HotkeyAction::UndoLastAction, w, cx);
        assert!(!s.state().unwrap().tasks()[0].completed);
    });
    assert_eq!(
        &*ports.events.borrow(),
        &[
            "register",
            "click-through:true",
            "hide",
            "show/topmost",
            "focus"
        ]
    );
}
#[gpui::test]
fn drop_revalidates_source_after_global_undo(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, vc) = open(cx, dir.path(), Duration::ZERO);
    let row = vc.debug_bounds("row-0").unwrap();
    scene.update_in(vc, |s, w, cx| {
        let original = s.state().unwrap().tasks().to_vec();
        let mut extra = original[0].clone();
        extra.id = TaskId(777.);
        let mut tasks = original.clone();
        tasks.push(extra.clone());
        s.dispatch_background(
            BackgroundCommand::CaptureUndo("new draggable task".into()),
            cx,
        );
        s.dispatch_background(
            BackgroundCommand::Replace {
                tasks,
                current_template: None,
            },
            cx,
        );
        s.start_drag(extra.id, row.origin, cx);
        s.drag_to(row.center(), cx);
        assert!(s.dragging.as_ref().unwrap().active);
        assert!(s.dragging.as_ref().unwrap().intent.is_some());
        s.global_hotkey(HotkeyAction::UndoLastAction, w, cx);
        assert!(
            s.dragging.is_some(),
            "global undo must not reset unrelated UI state"
        );
        let pending = s.pending.len();
        s.end_drag(row.center(), cx);
        assert!(s.dragging.is_none());
        assert_eq!(s.state().unwrap().tasks(), original.as_slice());
        assert_eq!(s.pending.len(), pending, "no stale drop write");
    });
}
#[gpui::test]
fn global_complete_and_accept_retain_duplicate_guards(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, vc) = open(cx, dir.path(), Duration::ZERO);
    scene.update_in(vc, |s, w, cx| {
        let task = s.state().unwrap().tasks()[0].clone();
        s.activate(Command::Edit(task.id), w, cx);
        s.dispatch_background(
            BackgroundCommand::Replace {
                tasks: vec![task.clone(), task.clone()],
                current_template: None,
            },
            cx,
        );
        let pending = s.pending.len();
        s.global_hotkey(HotkeyAction::CompleteNextTask, w, cx);
        s.activate(Command::Accept, w, cx);
        assert!(s.dialog.is_some());
        assert_eq!(s.state().unwrap().tasks(), &[task.clone(), task]);
        assert_eq!(s.pending.len(), pending);
    });
}
#[gpui::test]
fn register_summary_preserves_abort_position_and_release_warning(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, ports, vc) = open(cx, dir.path(), Duration::ZERO);
    let statuses = [
        ActionStatus::Active,
        ActionStatus::Inactive("OS false".into()),
        ActionStatus::ConversionAborted("non-ASCII".into()),
        ActionStatus::NotAttempted("conversion abort".into()),
        ActionStatus::NotAttempted("conversion abort".into()),
    ];
    let summary = RegisterSummary {
        actions: std::array::from_fn(|i| (HotkeyAction::ALL[i], statuses[i].clone())),
        retained: vec!["Ctrl+F1".into()],
    };
    *ports.summary.borrow_mut() = Some(summary.clone());
    *ports.release.borrow_mut() = ReleaseSummary {
        released: vec![],
        retained: vec![("Ctrl+F1".into(), "unregister failed".into())],
    };
    scene.update_in(vc, |s, w, cx| {
        s.register_committed();
        assert_eq!(s.overlay.registration.as_ref(), Some(&summary));
        s.settings_command(SettingsCommand::Open, w, cx);
        s.settings_command(
            SettingsCommand::Record(HotkeyAction::ToggleVisibility),
            w,
            cx,
        );
        assert!(s.settings.recorder.is_recording());
        assert!(
            s.settings
                .notice
                .as_ref()
                .unwrap()
                .contains("reserved/unavailable")
        );
        assert_eq!(s.overlay.release.as_ref(), Some(&*ports.release.borrow()));
    });
}
#[gpui::test]
fn delete_missing_and_dangling_trigger_choices_revalidate_without_filtering(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, _, vc) = open(cx, dir.path(), Duration::ZERO);
    scene.update_in(vc, |s, w, cx| {
        let owner = s.state().unwrap().tasks()[0].clone();
        s.activate(Command::Delete(owner.id), w, cx);
        s.dispatch_background(
            BackgroundCommand::Replace {
                tasks: vec![],
                current_template: None,
            },
            cx,
        );
        let pending = s.pending.len();
        s.activate(Command::Accept, w, cx);
        assert!(s.dialog.is_none());
        assert_eq!(s.pending.len(), pending);
        let mut b = owner.clone();
        b.id = TaskId(77.);
        b.mode = TaskMode::Background;
        let mut c = b.clone();
        c.id = TaskId(78.);
        s.dispatch_background(
            BackgroundCommand::Replace {
                tasks: vec![owner.clone(), b, c],
                current_template: None,
            },
            cx,
        );
        s.activate(Command::Triggers(owner.id), w, cx);
        for row in &mut s.trigger_rows {
            row.checked = true;
        }
        let expected: Vec<_> = s.trigger_rows.iter().map(|r| r.id).collect();
        s.dispatch_background(
            BackgroundCommand::Replace {
                tasks: vec![owner.clone()],
                current_template: None,
            },
            cx,
        );
        s.activate(Command::Accept, w, cx);
        assert_eq!(
            s.state().unwrap().find_task(owner.id).unwrap().triggers,
            expected
        );
        assert_eq!(expected, [TaskId(77.), TaskId(78.)]);
    });
}
