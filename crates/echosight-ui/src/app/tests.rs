use super::*;
use echosight_core::storage::{Stage, StorageHooks};
use gpui::{MouseButton, TestAppContext, VisualTestContext, point, px, size};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("missing {selector}"));
    cx.simulate_click(bounds.center(), Default::default());
    cx.run_until_parked();
}
fn open<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
) -> (Entity<TaskApp>, &'a mut VisualTestContext) {
    cx.update(crate::components::text_input::init);
    let (scene, cx) = cx.add_window_view(|window, cx| TaskApp::new(dir.into(), window, cx));
    cx.simulate_resize(size(px(500.), px(700.)));
    cx.run_until_parked();
    (scene, cx)
}
fn snapshot(scene: &Entity<TaskApp>, cx: &VisualTestContext) -> Vec<Task> {
    scene.read_with(cx, |s, _| s.state.as_ref().unwrap().tasks().to_vec())
}
async fn saved(scene: &Entity<TaskApp>, cx: &mut VisualTestContext) {
    cx.condition(scene, |s, _| !s.saving && s.pending.is_empty())
        .await;
    scene.read_with(cx, |s, _| assert!(s.error.is_none(), "{:?}", s.error));
}

#[gpui::test]
async fn add_subtask_edit_complete_delete_and_reload(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, vc) = open(cx, dir.path());
    vc.simulate_keystrokes("a l p h a");
    click(vc, "add-task");
    click(vc, "child-1");
    vc.simulate_keystrokes("c h i l d enter");
    vc.run_until_parked();
    assert_eq!(snapshot(&scene, vc)[1].children[0].text, "child");
    click(vc, "edit-1-0");
    vc.simulate_keystrokes("n e w enter");
    vc.run_until_parked();
    assert_eq!(snapshot(&scene, vc)[1].children[0].text, "new");
    click(vc, "toggle-1-0");
    scene.read_with(vc, |s, _| {
        assert_eq!(s.state.as_ref().unwrap().progress().completed, 2)
    });
    click(vc, "delete-1");
    vc.simulate_keystrokes("tab enter");
    assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
    vc.simulate_keystrokes("escape");
    assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
    click(vc, "delete-1");
    click(vc, "accept");
    assert_eq!(snapshot(&scene, vc).len(), 1);
    vc.simulate_keystrokes("ctrl-shift-z");
    assert_eq!(snapshot(&scene, vc).len(), 2);
    vc.simulate_keystrokes("ctrl-shift-y");
    assert_eq!(snapshot(&scene, vc).len(), 1);
    vc.simulate_keystrokes("ctrl-shift-z");
    saved(&scene, vc).await;
    let expected = snapshot(&scene, vc);
    let raw: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("current_tasks.json")).unwrap())
            .unwrap();
    assert_eq!(raw["tasks"][1]["children"][0]["text"], "new");
    let (reloaded, vc) = open(cx, dir.path());
    assert_eq!(snapshot(&reloaded, vc), expected);
}

#[gpui::test]
async fn long_delete_dialog_keeps_footer_visible_and_message_scrollable(cx: &mut TestAppContext) {
    for scale in [1., 1.5, 2.] {
        let dir = tempfile::tempdir().unwrap();
        let name = "word ".repeat(1000);
        seed(dir.path(), serde_json::json!([task(1, &name)]));
        let file = dir.path().join("current_tasks.json");
        let bytes = std::fs::read(&file).unwrap();
        let (scene, vc) = open(cx, dir.path());
        vc.simulate_scale_factor_change(scale);
        vc.simulate_keystrokes(&format!("{}space", "tab ".repeat(9)));
        assert!(scene.read_with(vc, |s, _| matches!(s.dialog, Some(Dialog::Delete(_)))));

        // Resize the live modal too: bounds are logical window pixels, not
        // physical screen pixels or a fixed maximum tuned to the default size.
        for dimensions in [size(px(500.), px(700.)), size(px(450.), px(600.))] {
            vc.simulate_resize(dimensions);
            vc.run_until_parked();
            let cancel = vc.debug_bounds("cancel").unwrap();
            let accept = vc.debug_bounds("accept").unwrap();
            for bounds in [cancel, accept] {
                assert!(
                    bounds.left() >= px(0.)
                        && bounds.right() <= dimensions.width
                        && bounds.top() >= px(0.)
                        && bounds.bottom() <= dimensions.height,
                    "offscreen dialog control at scale {scale}: {bounds:?} in {dimensions:?}"
                );
                assert!(bounds.size.width > px(0.) && bounds.size.height > px(0.));
            }
            scene.update_in(vc, |s, w, _| assert!(s.controls["cancel"].is_focused(w)));
            vc.simulate_keystrokes("shift-tab");
            scene.update_in(vc, |s, w, _| assert!(s.controls["accept"].is_focused(w)));
            vc.simulate_keystrokes("enter");
            assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
            vc.simulate_keystrokes("tab");
            scene.update_in(vc, |s, w, _| assert!(s.controls["cancel"].is_focused(w)));

            let panel = vc.debug_bounds("dialog-panel").unwrap();
            assert!(panel.top() >= px(0.) && panel.bottom() <= dimensions.height);
            let body = vc.debug_bounds("dialog-body").unwrap();
            let message = vc.debug_bounds("dialog-message").unwrap();
            assert!(body.size.height > px(0.) && body.bottom() <= cancel.top());
            assert!(message.size.height > body.size.height);
            let row = vc.debug_bounds("row-0").unwrap();
            vc.simulate_event(gpui::ScrollEvent {
                position: body.center(),
                delta: gpui::ScrollDelta::Lines(point(0., -8.)),
                ..Default::default()
            });
            vc.run_until_parked();
            assert!(vc.debug_bounds("dialog-message").unwrap().top() < message.top());
            vc.simulate_event(gpui::ScrollEvent {
                position: body.center(),
                delta: gpui::ScrollDelta::Lines(point(0., -1000.)),
                ..Default::default()
            });
            vc.run_until_parked();
            let end = vc.debug_bounds("dialog-message").unwrap();
            assert!(
                (end.bottom() - body.bottom()).abs() <= px(1.),
                "the entire warning must be reachable by scrolling"
            );
            assert_eq!(vc.debug_bounds("dialog-body").unwrap(), body);
            assert_eq!(vc.debug_bounds("cancel").unwrap(), cancel);
            assert_eq!(vc.debug_bounds("accept").unwrap(), accept);
            assert_eq!(
                vc.debug_bounds("row-0").unwrap(),
                row,
                "modal wheel must not scroll the task list"
            );
            // The test platform has no active AccessKit adapter. Inspect the
            // production element's accessibility payload, not a fabricated tree.
            let message = view::delete_message(&snapshot(&scene, vc)[0].text);
            let mut node = gpui::accesskit::Node::new(gpui::Role::Paragraph);
            gpui::Element::write_a11y_info(&message, &mut node);
            let warning = format!("Delete \"{name}\"? You can undo this with Ctrl+Shift+Z.");
            assert_eq!(node.label(), Some(warning.as_str()));
            assert_eq!(snapshot(&scene, vc)[0].text, name);
            assert_eq!(std::fs::read(&file).unwrap(), bytes);

            click(vc, "cancel");
            assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
            // Restored row control may be offscreen, but its keyboard command
            // still opens the production modal without a direct state mutation.
            vc.simulate_keystrokes("space");
            vc.run_until_parked();
        }
        vc.simulate_keystrokes("escape");
        assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
        vc.simulate_keystrokes("space");
        vc.run_until_parked();
        click(vc, "accept");
        assert!(snapshot(&scene, vc).is_empty());
        vc.simulate_keystrokes("ctrl-shift-z");
        saved(&scene, vc).await;
        assert_eq!(snapshot(&scene, vc)[0].text, name);
        let disk: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        assert_eq!(disk["tasks"][0]["text"], name);
    }
}

fn start_pointer_drag(cx: &mut VisualTestContext) -> gpui::Point<gpui::Pixels> {
    let row = cx.debug_bounds("row-0").unwrap();
    let target = cx.debug_bounds("row-1").unwrap();
    let destination = point(target.left() + px(40.), target.top() + px(10.));
    cx.simulate_mouse_down(
        point(row.left() + px(60.), row.top() + px(10.)),
        MouseButton::Left,
        Default::default(),
    );
    cx.simulate_mouse_move(destination, MouseButton::Left, Default::default());
    destination
}

#[gpui::test]
fn native_close_during_drag_is_dismissed_by_one_escape(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        serde_json::json!([task(1, "first"), task(2, "second")]),
    );
    let file = dir.path().join("current_tasks.json");
    let bytes = std::fs::read(&file).unwrap();
    let (scene, vc) = open(cx, dir.path());
    let before = snapshot(&scene, vc);
    let destination = start_pointer_drag(vc);
    assert!(scene.read_with(vc, |s, _| {
        s.dragging
            .as_ref()
            .is_some_and(|d| d.active && d.intent.is_some())
    }));
    assert!(!vc.simulate_close(), "native close must be intercepted");
    vc.run_until_parked();
    assert!(scene.read_with(vc, |s, _| matches!(s.dialog, Some(Dialog::Quit))));
    scene.update_in(vc, |s, w, _| assert!(s.controls["cancel"].is_focused(w)));
    vc.simulate_keystrokes("escape");
    assert!(
        scene.read_with(vc, |s, _| s.dialog.is_none()),
        "one Escape must dismiss the visible modal"
    );
    assert!(scene.read_with(vc, |s, _| s.dragging.is_none()));
    vc.simulate_mouse_move(destination, MouseButton::Left, Default::default());
    vc.simulate_mouse_up(destination, MouseButton::Left, Default::default());
    vc.run_until_parked();
    assert!(scene.read_with(vc, |s, _| s.dragging.is_none()));
    assert_eq!(snapshot(&scene, vc), before);
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[gpui::test]
fn every_modal_opening_discards_an_active_pointer_drag(cx: &mut TestAppContext) {
    for tabs in [5, 7, 8, 9] {
        // Quit, Child, Edit, Delete from the composer.
        let dir = tempfile::tempdir().unwrap();
        seed(
            dir.path(),
            serde_json::json!([task(1, "first"), task(2, "second")]),
        );
        let (scene, vc) = open(cx, dir.path());
        let before = snapshot(&scene, vc);
        let destination = start_pointer_drag(vc);
        assert!(scene.read_with(vc, |s, _| s.dragging.as_ref().is_some_and(|d| d.active)));
        vc.simulate_keystrokes(&format!("{}space", "tab ".repeat(tabs)));
        assert!(scene.read_with(vc, |s, _| s.dialog.is_some()));
        assert!(
            scene.read_with(vc, |s, _| s.dragging.is_none()),
            "modal opening must discard the drag"
        );
        vc.simulate_keystrokes("escape");
        assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
        vc.simulate_mouse_move(destination, MouseButton::Left, Default::default());
        vc.simulate_mouse_up(destination, MouseButton::Left, Default::default());
        vc.run_until_parked();
        assert!(scene.read_with(vc, |s, _| s.dragging.is_none()));
        assert_eq!(snapshot(&scene, vc), before);
    }
}

#[gpui::test]
fn tab_focus_and_danger_cancel_default(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, cx) = open(cx, dir.path());
    cx.simulate_keystrokes("tab");
    scene.update_in(cx, |s, w, _| assert!(s.controls["add-task"].is_focused(w)));
    click(cx, "delete-0");
    scene.update_in(cx, |s, w, _| assert!(s.controls["cancel"].is_focused(w)));
    cx.simulate_keystrokes("shift-tab");
    scene.update_in(cx, |s, w, _| assert!(s.controls["accept"].is_focused(w)));
    cx.simulate_keystrokes("enter");
    assert_eq!(snapshot(&scene, cx).len(), 1);
    cx.simulate_keystrokes("tab enter");
    assert!(scene.read_with(cx, |s, _| s.dialog.is_none()));
    scene.update_in(cx, |s, w, _| assert!(s.controls["delete-0"].is_focused(w)));
    cx.simulate_keystrokes("space tab space");
    cx.run_until_parked();
    assert!(snapshot(&scene, cx).is_empty());
}

fn drag(
    cx: &mut VisualTestContext,
    source: &'static str,
    target: &'static str,
    right: bool,
    above: bool,
) {
    let source = cx.debug_bounds(source).unwrap();
    let target = cx.debug_bounds(target).unwrap();
    let destination = point(
        target.left() + target.size.width * if right { 0.8 } else { 0.2 },
        target.top() + target.size.height * if above { 0.2 } else { 0.8 },
    );
    // Row title is the grip; controls must not start a drag.
    let start = point(source.left() + px(60.), source.top() + px(10.));
    cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(destination, MouseButton::Left, Default::default());
    cx.simulate_mouse_up(destination, MouseButton::Left, Default::default());
    cx.run_until_parked();
}

#[gpui::test]
async fn pointer_nest_promote_reorder_and_undo_persists(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, cx) = open(cx, dir.path());
    cx.simulate_keystrokes("a enter b enter");
    cx.run_until_parked();
    drag(cx, "row-2", "row-1", true, false);
    assert_eq!(snapshot(&scene, cx)[1].children[0].text, "b");
    drag(cx, "row-1-0", "row-0", false, true);
    assert_eq!(snapshot(&scene, cx)[0].text, "b");
    drag(cx, "row-0", "row-2", false, false);
    assert_eq!(snapshot(&scene, cx)[2].text, "b");
    cx.simulate_keystrokes("ctrl-shift-z");
    assert_eq!(snapshot(&scene, cx)[0].text, "b");
    saved(&scene, cx).await;
    let disk = persistence::load(
        &Storage::new(DataPaths::new(dir.path()).unwrap()),
        &DataPaths::new(dir.path()).unwrap(),
    )
    .unwrap();
    assert_eq!(disk.tasks, snapshot(&scene, cx));
}

fn seed(dir: &std::path::Path, tasks: serde_json::Value) {
    std::fs::write(
        dir.join("current_tasks.json"),
        serde_json::to_vec(
            &serde_json::json!({"tasks": tasks, "snapshots": [], "currentTemplate": null}),
        )
        .unwrap(),
    )
    .unwrap();
}
fn task(id: u32, text: &str) -> serde_json::Value {
    serde_json::json!({"id": id, "text": text, "completed": false, "createdAt": "2026-11-09T12:00:00.000Z"})
}
fn drag_point(
    cx: &mut VisualTestContext,
    source: &'static str,
    destination: gpui::Point<gpui::Pixels>,
) {
    let bounds = cx.debug_bounds(source).unwrap();
    cx.simulate_mouse_down(
        point(bounds.left() + px(60.), bounds.top() + px(10.)),
        MouseButton::Left,
        Default::default(),
    );
    cx.simulate_mouse_move(destination, MouseButton::Left, Default::default());
    cx.simulate_mouse_up(destination, MouseButton::Left, Default::default());
    cx.run_until_parked();
}

#[gpui::test]
fn pointer_rejects_self_descendant_crosslist_and_outside(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut parent = task(1, "parent");
    parent["children"] = serde_json::json!([task(2, "child")]);
    let mut background = task(3, "background");
    background["mode"] = "background".into();
    seed(dir.path(), serde_json::json!([parent, background]));
    let (scene, cx) = open(cx, dir.path());
    let before = snapshot(&scene, cx);
    let bytes = std::fs::read(dir.path().join("current_tasks.json")).unwrap();
    drag(cx, "row-0", "row-0", true, false);
    assert_eq!(snapshot(&scene, cx), before);
    drag(cx, "row-0", "row-0-0", true, false);
    assert_eq!(snapshot(&scene, cx), before);
    drag(cx, "row-0", "row-1", true, false);
    assert_eq!(snapshot(&scene, cx), before);
    drag_point(cx, "row-0", point(px(1.), px(1.)));
    assert_eq!(snapshot(&scene, cx), before);
    let list = cx.debug_bounds("rows").unwrap();
    drag_point(cx, "row-0", point(list.center().x, list.bottom() - px(5.)));
    assert_eq!(snapshot(&scene, cx), before);
    assert_eq!(
        std::fs::read(dir.path().join("current_tasks.json")).unwrap(),
        bytes
    );
}

#[gpui::test]
fn pointer_nearest_row_snaps_only_within_fifty_pixels(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), serde_json::json!([task(1, "a"), task(2, "b")]));
    let (scene, cx) = open(cx, dir.path());
    let target = cx.debug_bounds("row-1").unwrap();
    scene.read_with(cx, |s, _| {
        assert_eq!(
            s.rows.iter().find(|r| r.id == TaskId(2.)).unwrap().bounds,
            target
        )
    });
    let before = snapshot(&scene, cx);
    drag_point(
        cx,
        "row-0",
        point(target.left() + px(40.), target.center().y + px(50.)),
    );
    assert_eq!(snapshot(&scene, cx), before, "exactly 50px must not snap");
    let point = point(target.left() + px(40.), target.bottom() + px(1.));
    assert!(point.y - target.center().y < px(50.));
    drag_point(cx, "row-0", point);
    assert_eq!(snapshot(&scene, cx)[1].text, "a");
}

#[gpui::test]
fn duplicate_ids_are_rendered_but_ambiguous_actions_blocked(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), serde_json::json!([task(1, "a"), task(1, "b")]));
    let (scene, cx) = open(cx, dir.path());
    let before = snapshot(&scene, cx);
    assert!(cx.debug_bounds("row-0").is_some() && cx.debug_bounds("row-1").is_some());
    click(cx, "delete-1");
    click(cx, "toggle-1");
    drag(cx, "row-1", "row-0", true, false);
    assert!(scene.read_with(cx, |s, _| s.dialog.is_none()));
    assert_eq!(snapshot(&scene, cx), before);
}

#[gpui::test]
async fn complete_next_blocks_duplicate_ids_and_ancestry(cx: &mut TestAppContext) {
    // The selected leaf collides first, then its parent collides with root A.
    for (parent_id, leaf_id) in [(20, 1), (10, 2)] {
        let dir = tempfile::tempdir().unwrap();
        let mut completed_leaf = task(1, "completed leaf");
        completed_leaf["completed"] = true.into();
        let mut a = task(10, "A");
        a["children"] = serde_json::json!([completed_leaf]);
        let mut b = task(parent_id, "B");
        b["children"] = serde_json::json!([task(leaf_id, "incomplete leaf")]);
        seed(dir.path(), serde_json::json!([a, b]));
        let file = dir.path().join("current_tasks.json");
        let bytes = std::fs::read(&file).unwrap();
        let (scene, vc) = open(cx, dir.path());
        let before = snapshot(&scene, vc);

        // Dispatch through the real control before checking the handler directly.
        click(vc, "complete-next");
        assert_eq!(
            snapshot(&scene, vc),
            before,
            "ambiguous click must not mutate"
        );
        scene.read_with(vc, |s, _| {
            assert!(
                !s.tab_order
                    .iter()
                    .any(|(_, command)| matches!(command, Some(Command::Complete)))
            );
        });
        scene.update_in(vc, |s, window, cx| {
            s.activate(Command::Complete, window, cx)
        });
        saved(&scene, vc).await;
        assert_eq!(snapshot(&scene, vc), before, "handler must also block");
        scene.read_with(vc, |s, _| assert!(s.status.contains("ambiguous")));
        assert_eq!(std::fs::read(&file).unwrap(), bytes);
    }
}

#[gpui::test]
async fn background_priority_edit_and_complete_next(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut background = task(2, "background");
    background["mode"] = "background".into();
    seed(dir.path(), serde_json::json!([task(1, "main"), background]));
    let (scene, cx) = open(cx, dir.path());
    click(cx, "edit-1");
    cx.simulate_keystrokes("tab space tab tab enter");
    cx.run_until_parked();
    assert_eq!(
        snapshot(&scene, cx)[1]
            .background_options
            .as_ref()
            .unwrap()
            .priority,
        BackgroundPriority::High
    );
    click(cx, "complete-next");
    assert!(snapshot(&scene, cx)[0].completed);
    assert!(!snapshot(&scene, cx)[1].completed);
    saved(&scene, cx).await;
}

#[derive(Default)]
struct Probe {
    writes: Mutex<Vec<Value>>,
    fail: AtomicBool,
}
impl StorageHooks for Probe {
    fn before(&self, stage: Stage, path: &std::path::Path) -> std::io::Result<()> {
        if stage == Stage::Sync {
            self.writes
                .lock()
                .unwrap()
                .push(serde_json::from_slice(&std::fs::read(path)?).unwrap());
        }
        if stage == Stage::Replace && self.fail.load(Ordering::SeqCst) {
            return Err(std::io::Error::other("injected replacement failure"));
        }
        Ok(())
    }
}
fn open_probed<'a>(
    cx: &'a mut TestAppContext,
    dir: &std::path::Path,
    probe: Arc<Probe>,
) -> (Entity<TaskApp>, &'a mut VisualTestContext) {
    cx.update(crate::components::text_input::init);
    let paths = DataPaths::new(dir).unwrap();
    let storage = Storage::with_hooks(paths.clone(), probe);
    let (scene, cx) =
        cx.add_window_view(|window, cx| TaskApp::with_storage(Ok((storage, paths)), window, cx));
    cx.simulate_resize(size(px(500.), px(700.)));
    cx.run_until_parked();
    (scene, cx)
}

#[gpui::test]
async fn fifo_persists_each_mutation_without_debounce(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), serde_json::json!([]));
    let probe = Arc::new(Probe::default());
    let (scene, cx) = open_probed(cx, dir.path(), probe.clone());
    assert!(
        probe.writes.lock().unwrap().is_empty(),
        "no startup overwrite"
    );
    cx.simulate_keystrokes("a enter b enter c enter ctrl-shift-z ctrl-shift-y");
    saved(&scene, cx).await;
    let counts: Vec<_> = probe
        .writes
        .lock()
        .unwrap()
        .iter()
        .map(|v| v["tasks"].as_array().unwrap().len())
        .collect();
    assert_eq!(counts, vec![1, 2, 3, 2, 3]);
    let disk: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("current_tasks.json")).unwrap())
            .unwrap();
    assert_eq!(disk, *probe.writes.lock().unwrap().last().unwrap());
}

#[gpui::test]
async fn save_failure_blocks_later_writes_and_preserves_destination(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), serde_json::json!([task(1, "existing")]));
    let before = std::fs::read(dir.path().join("current_tasks.json")).unwrap();
    let probe = Arc::new(Probe::default());
    probe.fail.store(true, Ordering::SeqCst);
    let (scene, cx) = open_probed(cx, dir.path(), probe.clone());
    cx.simulate_keystrokes("a enter");
    cx.condition(&scene, |s, _| s.error.is_some()).await;
    assert!(!scene.read_with(cx, |s, _| s.writable()));
    cx.simulate_keystrokes("b enter ctrl-shift-z");
    click(cx, "toggle-0");
    assert_eq!(probe.writes.lock().unwrap().len(), 1);
    assert_eq!(
        std::fs::read(dir.path().join("current_tasks.json")).unwrap(),
        before
    );
    assert_eq!(
        snapshot(&scene, cx).len(),
        2,
        "unsaved edit retained visibly, not rolled back silently"
    );
}

#[gpui::test]
fn unsupported_load_disables_mutations_and_never_overwrites(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("current_tasks.json");
    std::fs::write(
        &file,
        br#"{"tasks":[{"id":"not-a-number"}],"snapshots":[]}"#,
    )
    .unwrap();
    let before = std::fs::read(&file).unwrap();
    let (scene, cx) = open(cx, dir.path());
    scene.read_with(cx, |s, _| {
        assert!(s.state.is_none());
        assert!(s.error.is_some());
    });
    cx.simulate_keystrokes("a enter");
    click(cx, "add-task");
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

#[gpui::test]
fn scrolling_keeps_drop_bounds_at_viewport_and_escape_cancels_drag(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        Value::Array((1..=15).map(|id| task(id, "row")).collect()),
    );
    let (scene, cx) = open(cx, dir.path());
    let list = cx.debug_bounds("rows").unwrap();
    cx.simulate_event(gpui::ScrollEvent {
        position: list.center(),
        delta: gpui::ScrollDelta::Lines(point(0., -8.)),
        ..Default::default()
    });
    cx.run_until_parked();
    let bounds = scene.read_with(cx, |s, _| s.list_bounds.unwrap());
    assert_eq!(
        bounds, list,
        "drop guard must be the viewport, not scrolled content"
    );
    let row = scene.read_with(cx, |s, _| {
        s.rows
            .iter()
            .find(|row| row.bounds.top() > list.top() && row.bounds.bottom() < list.bottom())
            .unwrap()
            .bounds
    });
    cx.simulate_mouse_down(
        point(row.left() + px(60.), row.top() + px(10.)),
        MouseButton::Left,
        Default::default(),
    );
    cx.simulate_mouse_move(row.center(), MouseButton::Left, Default::default());
    assert!(scene.read_with(cx, |s, _| s.dragging.as_ref().is_some_and(|d| d.active)));
    cx.simulate_keystrokes("escape");
    assert!(scene.read_with(cx, |s, _| s.dragging.is_none()));
}

#[gpui::test]
fn empty_edit_stays_open_with_local_error_and_cancel_preserves_data(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, cx) = open(cx, dir.path());
    let before = snapshot(&scene, cx);
    click(cx, "edit-0");
    cx.simulate_keystrokes("backspace enter");
    cx.run_until_parked();
    assert!(scene.read_with(cx, |s, _| s.dialog.is_some()));
    assert!(cx.debug_bounds("form-error").is_some());
    cx.simulate_keystrokes("escape");
    assert_eq!(snapshot(&scene, cx), before);
    assert!(!dir.path().join("current_tasks.json").exists());
}

#[gpui::test]
fn corrupt_recovery_is_reported_and_quarantined_bytes_are_preserved(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let broken = b"{not-json";
    std::fs::write(dir.path().join("current_tasks.json"), broken).unwrap();
    let (scene, cx) = open(cx, dir.path());
    scene.read_with(cx, |s, _| {
        assert!(
            s.error
                .as_ref()
                .unwrap()
                .contains("Corrupt tasks preserved")
        )
    });
    cx.simulate_keystrokes("a enter");
    let backups: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().contains(".corrupt-"))
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(std::fs::read(&backups[0]).unwrap(), broken);
    let recovered: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("current_tasks.json")).unwrap())
            .unwrap();
    assert_eq!(recovered["tasks"], serde_json::json!([]));
}

#[gpui::test]
fn quit_confirmation_is_cancel_default_and_does_not_mutate_tasks(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (scene, cx) = open(cx, dir.path());
    click(cx, "quit");
    scene.update_in(cx, |s, w, _| assert!(s.controls["cancel"].is_focused(w)));
    cx.simulate_keystrokes("tab enter");
    assert!(scene.read_with(cx, |s, _| matches!(s.dialog, Some(Dialog::Quit))));
    cx.simulate_keystrokes("escape");
    assert!(scene.read_with(cx, |s, _| s.dialog.is_none()));
    assert!(!dir.path().join("current_tasks.json").exists());
}

#[gpui::test]
async fn delete_blocks_ambiguous_descendants_without_touching_survivors(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut parent = task(10, "parent");
    parent["children"] = serde_json::json!([task(1, "descendant")]);
    let mut background = task(30, "background");
    background["mode"] = "background".into();
    background["triggers"] = serde_json::json!([1]);
    seed(
        dir.path(),
        serde_json::json!([parent, task(1, "surviving root"), background]),
    );
    let file = dir.path().join("current_tasks.json");
    let bytes = std::fs::read(&file).unwrap();
    let (scene, vc) = open(cx, dir.path());
    let before = snapshot(&scene, vc);
    click(vc, "delete-0");
    assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
    scene.read_with(vc, |s, _| {
        assert!(
            !s.tab_order
                .iter()
                .any(|(_, command)| matches!(command, Some(Command::Delete(TaskId(10.)))))
        );
    });
    scene.update_in(vc, |s, w, cx| {
        s.activate(Command::Delete(TaskId(10.)), w, cx)
    });
    assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
    // A stale confirmation must also pass the deletion guard.
    scene.update_in(vc, |s, _, cx| {
        s.dialog = Some(Dialog::Delete(TaskId(10.)));
        cx.notify();
    });
    vc.run_until_parked();
    click(vc, "accept");
    saved(&scene, vc).await;
    assert_eq!(snapshot(&scene, vc), before);
    assert_eq!(snapshot(&scene, vc)[2].triggers, vec![TaskId(1.)]);
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[gpui::test]
async fn unchanged_loaded_long_name_save_preserves_text(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let name = "é".repeat(101);
    seed(dir.path(), serde_json::json!([task(1, &name)]));
    let file = dir.path().join("current_tasks.json");
    let bytes = std::fs::read(&file).unwrap();
    let (scene, vc) = open(cx, dir.path());
    click(vc, "edit-0");
    click(vc, "accept");
    saved(&scene, vc).await;
    assert_eq!(snapshot(&scene, vc)[0].text, name);
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[gpui::test]
async fn long_background_name_priority_only_save_preserves_text(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let name = "é".repeat(101);
    let mut background = task(1, &name);
    background["mode"] = "background".into();
    seed(dir.path(), serde_json::json!([background]));
    let (scene, vc) = open(cx, dir.path());
    click(vc, "edit-0");
    click(vc, "priority");
    click(vc, "accept");
    saved(&scene, vc).await;
    let tasks = snapshot(&scene, vc);
    assert_eq!(tasks[0].text, name);
    assert_eq!(
        tasks[0].background_options.as_ref().unwrap().priority,
        BackgroundPriority::High
    );
    let disk: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("current_tasks.json")).unwrap())
            .unwrap();
    assert_eq!(disk["tasks"][0]["text"], name);
}

#[gpui::test]
async fn recreated_editor_enforces_utf16_insertion_limit(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let name = "a".repeat(101);
    seed(dir.path(), serde_json::json!([task(1, &name)]));
    let (scene, vc) = open(cx, dir.path());
    click(vc, "edit-0");
    vc.simulate_keystrokes("right x");
    assert_eq!(
        scene.read_with(vc, |s, cx| s.editor.read(cx).text().to_owned()),
        name
    );
    vc.simulate_keystrokes("ctrl-a");
    vc.simulate_input(&format!("{}😀z", "a".repeat(98)));
    vc.simulate_keystrokes("enter");
    saved(&scene, vc).await;
    assert_eq!(
        snapshot(&scene, vc)[0].text,
        format!("{}😀", "a".repeat(98))
    );
    click(vc, "edit-0");
    vc.simulate_keystrokes("escape");
    assert!(scene.read_with(vc, |s, _| s.dialog.is_none()));
}
