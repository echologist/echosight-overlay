use super::*;
mod triggers;
use std::time::Duration;
const EPOCH: i64 = 1_800_000_000_000;

#[gpui::test]
async fn parity_complete_feedback_uses_actual_main_root(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut background = bg(20, true, 0.);
    background["children"] = serde_json::json!([task(21, "Skipped child")]);
    seed(
        dir.path(),
        serde_json::json!([background, task(1, "Actual completion")]),
    );
    let probe = Arc::new(Probe::default());
    let events = Arc::new(Mutex::new(vec![]));
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), events.clone());
    click(vc, "complete-next");
    assert!(!snapshot(&scene, vc)[0].children[0].completed);
    assert!(snapshot(&scene, vc)[1].completed);
    scene.read_with(vc, |s, _| {
        assert_eq!(
            s.feedback.as_deref(),
            Some("✓ Completed: Actual completion")
        )
    });
    saved(&scene, vc).await;
    assert_eq!(probe.writes.lock().unwrap().len(), 1);
    assert_eq!(
        *events.lock().unwrap(),
        vec![TaskRuntimeEvent::Sound("taskCompleted")]
    );
}

#[gpui::test]
async fn parity_complete_without_main_root_has_no_feedback(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut background = bg(20, true, 0.);
    background["children"] = serde_json::json!([task(21, "Skipped child")]);
    seed(dir.path(), serde_json::json!([background]));
    let probe = Arc::new(Probe::default());
    let events = Arc::new(Mutex::new(vec![]));
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), events.clone());
    let before = snapshot(&scene, vc);
    click(vc, "complete-next");
    assert_eq!(snapshot(&scene, vc), before);
    scene.read_with(vc, |s, _| assert!(s.feedback.is_none(), "{:?}", s.feedback));
    saved(&scene, vc).await;
    assert!(probe.writes.lock().unwrap().is_empty());
    assert!(events.lock().unwrap().is_empty());
}

#[gpui::test]
async fn parity_activated_parent_renders_dormant_child(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut main = task(1, "Activate parent only");
    main["triggers"] = serde_json::json!([20]);
    let mut background = bg(20, false, 0.);
    background["children"] = serde_json::json!([bg(21, false, 0.)]);
    seed(dir.path(), serde_json::json!([main, background]));
    let (scene, vc) = runtime(cx, dir.path(), Arc::default(), Arc::default());
    assert!(vc.debug_bounds("row-1").is_none());
    assert!(vc.debug_bounds("row-1-0").is_none());
    click(vc, "complete-next");
    assert!(snapshot(&scene, vc)[1].activated);
    assert!(!snapshot(&scene, vc)[1].children[0].activated);
    assert!(vc.debug_bounds("row-1").is_some());
    assert!(vc.debug_bounds("row-1-0").is_some());
    saved(&scene, vc).await;
}

#[gpui::test]
fn parity_root_filter_keeps_mixed_mode_descendants(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut main = task(1, "Main parent");
    main["children"] = serde_json::json!([bg(21, false, 0.)]);
    let mut background = bg(20, true, 0.);
    background["children"] = serde_json::json!([task(22, "Legacy main child")]);
    seed(
        dir.path(),
        serde_json::json!([main, background, bg(23, false, 0.)]),
    );
    let (scene, vc) = runtime(cx, dir.path(), Arc::default(), Arc::default());
    assert!(vc.debug_bounds("row-0-0").is_some());
    assert!(vc.debug_bounds("row-1-0").is_some());
    assert!(vc.debug_bounds("row-2").is_none());
    assert!(!snapshot(&scene, vc)[0].children[0].activated);
}

#[gpui::test]
fn failed_load_never_restarts_expired_backgrounds_or_saves(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut invalid = bg(21, true, -1.);
    invalid["text"] = serde_json::json!({"unsupported":"not a string"});
    seed(dir.path(), serde_json::json!([bg(20, true, -1.), invalid]));
    let before = std::fs::read(dir.path().join("current_tasks.json")).unwrap();
    let probe = Arc::new(Probe::default());
    let events = Arc::new(Mutex::new(vec![]));
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), events.clone());
    assert!(scene.read_with(vc, |s, _| s.state().is_none()
        && s.error.is_some()
        && s.expiry_task.is_none()));
    click(vc, "complete-next");
    vc.simulate_keystrokes("a enter ctrl-shift-z");
    advance(vc, 60000);
    assert!(probe.writes.lock().unwrap().is_empty());
    assert!(events.lock().unwrap().is_empty());
    assert_eq!(
        std::fs::read(dir.path().join("current_tasks.json")).unwrap(),
        before
    );
}

fn bg(id: u32, active: bool, minutes: f64) -> Value {
    let mut task = task(id, &format!("Background {id}"));
    task["mode"] = "background".into();
    task["activated"] = active.into();
    task["backgroundOptions"] =
        serde_json::json!({"expiresAfterMinutes": minutes, "priority": "high"});
    task
}
fn advance(cx: &mut VisualTestContext, ms: u64) {
    cx.background_executor
        .advance_clock(Duration::from_millis(ms));
    cx.run_until_parked();
}
fn runtime<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
    probe: Arc<Probe>,
    events: Arc<Mutex<Vec<TaskRuntimeEvent>>>,
) -> (Entity<TaskApp>, &'a mut VisualTestContext) {
    cx.update(crate::components::text_input::init);
    let paths = DataPaths::new(dir).unwrap();
    let storage = Storage::with_hooks(paths.clone(), probe);
    let (scene, vc) = cx.add_window_view(|w, cx| {
        let clock = TaskClock::anchored(cx.background_executor().clone(), EPOCH, || 0.017);
        TaskApp::with_runtime(
            Ok((storage, paths)),
            clock,
            Arc::new(move |e| events.lock().unwrap().push(e)),
            w,
            cx,
        )
    });
    vc.simulate_resize(size(px(500.), px(700.)));
    vc.run_until_parked();
    (scene, vc)
}

#[gpui::test]
async fn trigger_activation_feedback_sound_and_real_expiry(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut main = task(1, "Complete me");
    main["triggers"] = serde_json::json!([20]);
    seed(dir.path(), serde_json::json!([main, bg(20, false, 1.)]));
    let probe = Arc::new(Probe::default());
    let events = Arc::new(Mutex::new(vec![]));
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), events.clone());
    click(vc, "complete-next");
    assert!(snapshot(&scene, vc)[1].activated);
    assert_eq!(
        snapshot(&scene, vc)[1].activated_at.as_deref(),
        Some("2027-01-15T08:00:00.000Z")
    );
    scene.read_with(vc, |s, _| {
        assert_eq!(s.feedback.as_deref(), Some("✓ Completed: Complete me"));
        assert_eq!(
            s.activation_toast.as_deref(),
            Some("1 background task activated!")
        );
    });
    assert_eq!(
        *events.lock().unwrap(),
        vec![
            TaskRuntimeEvent::Activated {
                count: 1,
                message: "1 background task activated!".into()
            },
            TaskRuntimeEvent::Sound("backgroundActivated"),
            TaskRuntimeEvent::Sound("taskCompleted")
        ]
    );
    advance(vc, 2000);
    assert!(scene.read_with(vc, |s, _| s.feedback.is_none()));
    advance(vc, 1300);
    assert!(scene.read_with(vc, |s, _| s.activation_toast.is_none()));
    advance(vc, 56700);
    assert!(!snapshot(&scene, vc)[1].activated);
    saved(&scene, vc).await;
    assert_eq!(probe.writes.lock().unwrap().len(), 2);
    assert_eq!(
        events.lock().unwrap().last(),
        Some(&TaskRuntimeEvent::Log(
            "Background task \"Background 20\" expired".into()
        ))
    );
}

#[gpui::test]
async fn move_edit_delete_undo_redo_preserve_deadlines(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut first = bg(20, true, 1.);
    first["activatedAt"] = "2027-01-15T08:00:00.000Z".into();
    first["children"] = serde_json::json!([bg(22, true, 1.)]);
    seed(dir.path(), serde_json::json!([first, bg(21, true, 2.)]));
    let (scene, vc) = runtime(cx, dir.path(), Arc::default(), Arc::default());
    let deadline = scene.read_with(vc, |s, _| s.background.as_ref().unwrap().next_due_ms());
    advance(vc, 10000);
    drag(vc, "row-0", "row-1", true, false);
    assert_eq!(snapshot(&scene, vc)[0].children[0].id, TaskId(20.));
    assert_eq!(
        scene.read_with(vc, |s, _| s.background.as_ref().unwrap().next_due_ms()),
        deadline
    );
    click(vc, "edit-0-0");
    vc.simulate_keystrokes("r e n a m e d enter");
    click(vc, "delete-0-0");
    click(vc, "accept");
    assert_eq!(
        scene.read_with(vc, |s, _| s.background.as_ref().unwrap().timers().len()),
        1
    );
    vc.simulate_keystrokes("ctrl-shift-z ctrl-shift-y ctrl-shift-z");
    vc.run_until_parked();
    advance(vc, 50000);
    let tasks = snapshot(&scene, vc);
    assert!(tasks[0].activated);
    assert!(!tasks[0].children[0].activated);
    assert_eq!(tasks[0].children[0].text, "renamed");
    // Missing activatedAt on the nested child restarts a full duration on undo,
    // source quirk. Moving/editing alone did not reset it.
    assert!(tasks[0].children[0].children[0].activated);
    advance(vc, 10000);
    assert!(!snapshot(&scene, vc)[0].children[0].children[0].activated);
    saved(&scene, vc).await;
}

#[gpui::test]
async fn immediate_expirations_queue_distinct_snapshot_payloads(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        serde_json::json!([bg(20, true, -1.), bg(21, true, -1.)]),
    );
    let probe = Arc::new(Probe::default());
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), Arc::default());
    saved(&scene, vc).await;
    let writes = probe.writes.lock().unwrap();
    assert_eq!(writes.len(), 2);
    assert_eq!(writes[0]["tasks"][0]["activated"], false);
    assert_eq!(writes[0]["tasks"][1]["activated"], true);
    assert_eq!(writes[1]["tasks"][1]["activated"], false);
    assert_eq!(writes[0]["snapshots"].as_array().unwrap().len(), 1);
    assert_eq!(writes[1]["snapshots"].as_array().unwrap().len(), 2);
}

#[gpui::test]
async fn cancel_replace_stale_wakeup_and_shutdown(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), serde_json::json!([bg(20, true, 1.)]));
    let probe = Arc::new(Probe::default());
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), Arc::default());
    let stale = scene.read_with(vc, |s, _| s.expiry_generation);
    let replacement = snapshot(&scene, vc);
    scene.update(vc, |s, cx| {
        s.dispatch_background(BackgroundCommand::CancelAll, cx);
        s.dispatch_background(
            BackgroundCommand::Replace {
                tasks: replacement,
                current_template: Some("replacement".into()),
            },
            cx,
        );
        s.expiry_woke(stale, cx);
    });
    advance(vc, 60000);
    assert!(snapshot(&scene, vc)[0].activated);
    assert!(probe.writes.lock().unwrap().is_empty());
    scene.update(vc, |s, cx| {
        s.dispatch_background(BackgroundCommand::Restart, cx);
    });
    let weak = scene.downgrade();
    vc.update(|w, _| w.remove_window());
    drop(scene);
    vc.run_until_parked();
    assert!(weak.upgrade().is_none(), "timer must not retain its entity");
    advance(vc, 60000);
    assert!(probe.writes.lock().unwrap().is_empty());
}

#[gpui::test]
async fn expiry_save_failure_blocks_future_mutations(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        serde_json::json!([bg(20, true, 1.), bg(21, true, 2.)]),
    );
    let before = std::fs::read(dir.path().join("current_tasks.json")).unwrap();
    let probe = Arc::new(Probe::default());
    probe.fail.store(true, Ordering::SeqCst);
    let (scene, vc) = runtime(cx, dir.path(), probe.clone(), Arc::default());
    advance(vc, 60000);
    vc.condition(&scene, |s, _| s.error.is_some()).await;
    assert!(scene.read_with(vc, |s, _| s.error.as_deref().unwrap().contains("NOT saved")));
    advance(vc, 60000);
    assert!(snapshot(&scene, vc)[1].activated);
    assert_eq!(probe.writes.lock().unwrap().len(), 1);
    assert_eq!(
        std::fs::read(dir.path().join("current_tasks.json")).unwrap(),
        before
    );
}
