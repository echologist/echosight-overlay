use super::*;

#[gpui::test]
fn duplicate_trigger_choices_are_guarded_in_dispatch(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        serde_json::json!([task(1, "Main"), bg(20, false, 1.), bg(20, false, 2.)]),
    );
    let (scene, vc) = runtime(cx, dir.path(), Arc::default(), Arc::default());
    click(vc, "triggers-0");
    click(vc, "trigger-choice-0");
    assert!(scene.read_with(vc, |s, _| !s.trigger_rows[0].checked));
    scene.update_in(vc, |s, w, cx| s.activate(Command::TriggerChoice(0), w, cx));
    assert!(scene.read_with(vc, |s, _| !s.trigger_rows[0].checked));
}

/// Dispatched counterpart of rendererTaskEditing.test.ts: exact record fields
/// survive nested/active/dormant edits, with four separate persistence callbacks.
#[gpui::test]
async fn renderer_task_editing_source_scenario(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut child = task(2, "Nested objective");
    child["completed"] = true.into();
    child["triggers"] = serde_json::json!([20]);
    let mut parent = task(1, "Parent objective");
    parent["children"] = serde_json::json!([child]);
    let mut active = bg(20, true, 15.);
    active["completed"] = true.into();
    active["triggers"] = serde_json::json!([77]);
    active["activatedAt"] = "2026-07-25T03:00:00.000Z".into();
    active["backgroundOptions"]["priority"] = "normal".into();
    let mut dormant = bg(30, false, 1.);
    dormant["backgroundOptions"] =
        serde_json::json!({"expiresAfterMinutes":null,"priority":"normal"});
    seed(dir.path(), serde_json::json!([parent, active, dormant]));
    let probe = Arc::new(Probe::default());
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), Arc::default());
    let before = snapshot(&scene, vc);
    click(vc, "edit-0-0");
    vc.simulate_input("Cancelled change");
    vc.simulate_keystrokes("escape");
    assert_eq!(snapshot(&scene, vc), before);
    assert!(probe.writes.lock().unwrap().is_empty());
    click(vc, "edit-0-0");
    vc.simulate_input("  Renamed objective  ");
    click(vc, "accept");
    let mut expected_child = before[0].children[0].clone();
    expected_child.text = "Renamed objective".into();
    assert_eq!(snapshot(&scene, vc)[0].children[0], expected_child);
    click(vc, "edit-1");
    vc.simulate_input("  Watch new timer  ");
    click(vc, "priority");
    click(vc, "accept");
    let mut expected_active = before[1].clone();
    expected_active.text = "Watch new timer".into();
    expected_active
        .background_options
        .as_mut()
        .unwrap()
        .priority = BackgroundPriority::High;
    assert_eq!(snapshot(&scene, vc)[1], expected_active);
    click(vc, "triggers-0-0");
    click(vc, "trigger-choice-1");
    click(vc, "trigger-edit-1");
    assert!(scene.read_with(vc, |s, _| matches!(
        s.dialog,
        Some(Dialog::Edit(TaskId(30.), true))
    ) && s.modal_stack.len() == 1));
    vc.simulate_input("  Renamed dormant timer  ");
    click(vc, "priority");
    click(vc, "accept");
    let mut expected_dormant = before[2].clone();
    expected_dormant.text = "Renamed dormant timer".into();
    expected_dormant
        .background_options
        .as_mut()
        .unwrap()
        .priority = BackgroundPriority::High;
    assert_eq!(snapshot(&scene, vc)[2], expected_dormant);
    assert!(scene.read_with(vc, |s, _| s.trigger_rows[1].checked));
    click(vc, "accept");
    saved(&scene, vc).await;
    assert_eq!(
        snapshot(&scene, vc)[0].children[0].triggers,
        vec![TaskId(20.), TaskId(30.)]
    );
    assert_eq!(probe.writes.lock().unwrap().len(), 4);
}

#[gpui::test]
async fn trigger_modal_scroll_trap_and_nested_escape(cx: &mut TestAppContext) {
    for scale in [1., 1.5, 2.] {
        let dir = tempfile::tempdir().unwrap();
        let mut tasks = vec![task(1, "Main")];
        tasks.extend((20..35).map(|id| {
            let mut t = bg(id, false, 1.);
            t["text"] = "long dormant description ".repeat(10).into();
            t
        }));
        seed(dir.path(), tasks.into());
        let (scene, vc) = runtime(cx, dir.path(), Arc::default(), Arc::default());
        vc.simulate_scale_factor_change(scale);
        click(vc, "triggers-0");
        assert!(scene.read_with(vc, |s, _| matches!(
            s.dialog,
            Some(Dialog::Triggers(TaskId(1.)))
        )));
        for dimensions in [size(px(500.), px(700.)), size(px(450.), px(600.))] {
            vc.simulate_resize(dimensions);
            vc.run_until_parked();
            let body = vc.debug_bounds("dialog-body").unwrap();
            let row = vc.debug_bounds("row-0").unwrap();
            let cancel = vc.debug_bounds("cancel").unwrap();
            let accept = vc.debug_bounds("accept").unwrap();
            assert!(cancel.top() >= body.bottom() && cancel.bottom() <= dimensions.height);
            assert!(accept.left() >= px(0.) && accept.right() <= dimensions.width);
            vc.simulate_event(gpui::ScrollEvent {
                position: body.center(),
                delta: gpui::ScrollDelta::Lines(point(0., -10000.)),
                ..Default::default()
            });
            vc.run_until_parked();
            assert_eq!(
                vc.debug_bounds("row-0").unwrap(),
                row,
                "occluded background list must not scroll"
            );
            assert_eq!(vc.debug_bounds("cancel").unwrap(), cancel);
            let input = vc.debug_bounds("new-background-input").unwrap();
            assert!(input.top() >= body.top() && input.bottom() <= body.bottom());
            click(vc, "new-background-input");
            vc.simulate_keystrokes("tab tab tab tab tab");
            scene.update_in(vc, |s, w, _| {
                assert!(s.controls["trigger-choice-0"].is_focused(w))
            });
            vc.simulate_keystrokes("tab space");
            vc.run_until_parked();
            assert!(scene.read_with(vc, |s, _| matches!(s.dialog, Some(Dialog::Edit(_, true)))));
            vc.simulate_keystrokes("shift-tab");
            scene.update_in(vc, |s, w, _| assert!(s.controls["accept"].is_focused(w)));
            vc.simulate_keystrokes("tab");
            scene.update_in(vc, |s, w, cx| {
                assert!(s.editor.focus_handle(cx).is_focused(w))
            });
            vc.simulate_keystrokes("escape");
            assert!(scene.read_with(vc, |s, _| matches!(s.dialog, Some(Dialog::Triggers(_)))));
            scene.update_in(vc, |s, w, _| {
                assert!(s.controls["trigger-edit-0"].is_focused(w))
            });
        }
        click(vc, "cancel");
        assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
        saved(&scene, vc).await;
    }
}

#[gpui::test]
async fn trigger_input_limit_and_nested_loaded_name_preservation(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let name = "😀".repeat(70);
    let mut dormant = bg(20, false, 3.);
    dormant["text"] = name.clone().into();
    seed(dir.path(), serde_json::json!([task(1, "Main"), dormant]));
    let (scene, vc) = runtime(cx, dir.path(), Arc::default(), Arc::default());
    click(vc, "triggers-0");
    click(vc, "trigger-edit-0");
    click(vc, "priority");
    click(vc, "accept");
    assert_eq!(snapshot(&scene, vc)[1].text, name);
    click(vc, "new-background-input");
    vc.simulate_input(&format!("{}😀z", "a".repeat(98)));
    vc.simulate_keystrokes("enter");
    assert_eq!(
        snapshot(&scene, vc)[2].text,
        format!("{}😀", "a".repeat(98))
    );
    click(vc, "cancel");
    saved(&scene, vc).await;
}

#[gpui::test]
async fn background_history_is_deep_capped_and_new_change_clears_redo(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut parent = bg(20, true, 1.);
    parent["children"] = serde_json::json!([bg(21, true, 1.)]);
    seed(dir.path(), serde_json::json!([parent]));
    let probe = Arc::new(Probe::default());
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), Arc::default());
    for _ in 0..22 {
        click(vc, "edit-0-0");
        click(vc, "priority");
        click(vc, "accept");
    }
    saved(&scene, vc).await;
    let writes_before = probe.writes.lock().unwrap().len();
    assert_eq!(writes_before, 22);
    vc.simulate_keystrokes(&"ctrl-shift-z ".repeat(21));
    saved(&scene, vc).await;
    assert_eq!(
        probe.writes.lock().unwrap().len(),
        42,
        "only twenty undo entries"
    );
    vc.simulate_keystrokes(&"ctrl-shift-y ".repeat(21));
    saved(&scene, vc).await;
    assert_eq!(probe.writes.lock().unwrap().len(), 62);
    vc.simulate_keystrokes("ctrl-shift-z");
    click(vc, "edit-0-0");
    vc.simulate_keystrokes("n e w enter");
    vc.simulate_keystrokes("ctrl-shift-y");
    saved(&scene, vc).await;
    let writes = probe.writes.lock().unwrap();
    assert_eq!(writes.len(), 64);
    assert!(
        writes
            .iter()
            .all(|v| v["snapshots"].as_array().unwrap().len() <= 5)
    );
    assert_eq!(
        writes.last().unwrap()["snapshots"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert_eq!(
        writes[0]["tasks"][0]["children"][0]["text"], "Background 21",
        "history/save payloads must not share mutable descendants"
    );
    assert_eq!(snapshot(&scene, vc)[0].children[0].text, "new");
}

#[gpui::test]
async fn expired_focused_row_restores_local_history_keys(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut main = task(1, "Main");
    main["triggers"] = serde_json::json!([20]);
    seed(dir.path(), serde_json::json!([main, bg(20, false, 1.)]));
    let (scene, vc) = runtime(cx, dir.path(), Arc::default(), Arc::default());
    click(vc, "complete-next");
    click(vc, "triggers-1");
    vc.simulate_keystrokes("escape");
    scene.update_in(vc, |s, w, _| {
        assert!(s.controls["triggers-1"].is_focused(w))
    });
    advance(vc, 60000);
    assert!(vc.debug_bounds("row-1").is_none());
    vc.simulate_keystrokes("ctrl-shift-z");
    vc.run_until_parked();
    assert!(
        !snapshot(&scene, vc)[0].completed,
        "local undo survives the focused row's expiry"
    );
    saved(&scene, vc).await;
}

#[gpui::test]
async fn dormant_nested_edit_preserves_selection_cancel_and_focus(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut main = task(1, "Main objective");
    main["triggers"] = serde_json::json!([20]);
    let mut dormant = bg(20, false, 2.);
    dormant["children"] = serde_json::json!([bg(21, false, 1.)]);
    seed(dir.path(), serde_json::json!([main, dormant]));
    let probe = Arc::new(Probe::default());
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), Arc::default());
    assert!(
        vc.debug_bounds("row-1").is_none(),
        "dormant roots belong only in Configure Triggers"
    );
    assert!(vc.debug_bounds("background-heading").is_none());
    click(vc, "triggers-0");
    assert!(scene.read_with(vc, |s, _| matches!(
        s.dialog,
        Some(Dialog::Triggers(TaskId(1.)))
    )));
    scene.update_in(vc, |s, w, cx| {
        assert!(s.editor.focus_handle(cx).is_focused(w))
    });
    assert!(vc.debug_bounds("trigger-choice-0").is_some());
    assert!(
        vc.debug_bounds("trigger-choice-1").is_some(),
        "nested dormant choice"
    );
    click(vc, "trigger-edit-0");
    assert!(scene.read_with(vc, |s, _| matches!(
        s.dialog,
        Some(Dialog::Edit(TaskId(20.), true))
    )));
    vc.simulate_keystrokes("c h a n g e d");
    click(vc, "priority");
    click(vc, "accept");
    assert!(scene.read_with(vc, |s, _| matches!(
        s.dialog,
        Some(Dialog::Triggers(TaskId(1.)))
    )));
    scene.update_in(vc, |s, w, _| {
        assert!(s.controls["trigger-edit-0"].is_focused(w))
    });
    assert_eq!(snapshot(&scene, vc)[1].text, "changed");
    assert_eq!(
        snapshot(&scene, vc)[1]
            .background_options
            .as_ref()
            .unwrap()
            .expires_after_minutes,
        Some(2.)
    );
    assert_eq!(
        snapshot(&scene, vc)[1]
            .background_options
            .as_ref()
            .unwrap()
            .priority,
        BackgroundPriority::Normal
    );
    assert_eq!(snapshot(&scene, vc)[0].triggers, vec![TaskId(20.)]);
    click(vc, "trigger-choice-0");
    click(vc, "trigger-edit-1");
    vc.simulate_keystrokes("x escape");
    assert_eq!(snapshot(&scene, vc)[1].children[0].text, "Background 21");
    click(vc, "accept");
    assert!(snapshot(&scene, vc)[0].triggers.is_empty());
    assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
    saved(&scene, vc).await;
    assert_eq!(probe.writes.lock().unwrap().len(), 2);
    vc.simulate_keystrokes("ctrl-shift-z ctrl-shift-z");
    assert_eq!(snapshot(&scene, vc)[1].text, "Background 20");
    assert_eq!(snapshot(&scene, vc)[0].triggers, vec![TaskId(20.)]);
    saved(&scene, vc).await;
}

#[gpui::test]
async fn create_background_validation_refresh_cancel_and_ordered_save(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut main = task(1, "Main");
    main["triggers"] = serde_json::json!([21, 20]);
    seed(
        dir.path(),
        serde_json::json!([main, bg(20, false, 1.), bg(21, false, 2.)]),
    );
    let probe = Arc::new(Probe::default());
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), Arc::default());
    click(vc, "triggers-0");
    vc.simulate_keystrokes("enter");
    assert_eq!(
        scene.read_with(vc, |s, _| s.form_error.clone()).as_deref(),
        Some("Please enter a background task description.")
    );
    assert_eq!(snapshot(&scene, vc).len(), 3);
    // Creation refreshes selection from the saved task, discarding unchecked
    // unsaved boxes; the newly created task is NOT automatically selected.
    click(vc, "trigger-choice-0");
    click(vc, "priority");
    click(vc, "new-background-input");
    vc.simulate_keystrokes("n e w enter");
    assert_eq!(snapshot(&scene, vc).len(), 4);
    assert!(!snapshot(&scene, vc)[3].activated);
    assert_eq!(
        snapshot(&scene, vc)[3]
            .background_options
            .as_ref()
            .unwrap()
            .priority,
        BackgroundPriority::High
    );
    assert_eq!(
        snapshot(&scene, vc)[3]
            .background_options
            .as_ref()
            .unwrap()
            .expires_after_minutes,
        None
    );
    click(vc, "accept");
    assert_eq!(
        snapshot(&scene, vc)[0].triggers,
        vec![TaskId(20.), TaskId(21.)],
        "DOM order, not previous selection order"
    );
    saved(&scene, vc).await;
    assert_eq!(probe.writes.lock().unwrap().len(), 2);
    click(vc, "triggers-0");
    click(vc, "accept");
    saved(&scene, vc).await;
    assert_eq!(
        probe.writes.lock().unwrap().len(),
        2,
        "ordered equality means no redundant save"
    );
    click(vc, "triggers-0");
    click(vc, "trigger-choice-2");
    vc.simulate_keystrokes("escape");
    assert_eq!(
        snapshot(&scene, vc)[0].triggers,
        vec![TaskId(20.), TaskId(21.)]
    );
    click(vc, "complete-next");
    assert!(vc.debug_bounds("background-heading").is_some());
    assert!(vc.debug_bounds("row-1").is_some());
    assert!(vc.debug_bounds("row-3").is_none());
    assert!(vc.debug_bounds("high-1").is_some());
    advance(vc, 60000);
    assert!(vc.debug_bounds("row-1").is_none());
    assert!(vc.debug_bounds("row-2").is_some());
    saved(&scene, vc).await;
}
