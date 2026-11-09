use super::*;
use crate::components::text_input;
use gpui::{ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, size};

fn open(cx: &mut TestAppContext) -> (Entity<InputSpike>, &mut VisualTestContext) {
    cx.update(text_input::init);
    let (scene, cx) = cx.add_window_view(InputSpike::new);
    cx.simulate_resize(size(px(500.), px(700.)));
    cx.run_until_parked();
    (scene, cx)
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds = cx.debug_bounds(selector).expect("control is rendered");
    cx.simulate_click(bounds.center(), Default::default());
}

#[gpui::test]
fn tab_select_slider_switch_and_submit(cx: &mut TestAppContext) {
    let (scene, cx) = open(cx);
    cx.simulate_keystrokes("a b enter tab down tab right right tab space");
    scene.read_with(cx, |scene, _| {
        assert_eq!(scene.last_event, "Submitted: ab");
        assert_eq!(scene.priority, 1);
        assert_eq!(scene.volume, 62);
        assert!(scene.sounds);
    });
    cx.simulate_keystrokes("shift-tab home left");
    assert_eq!(scene.read_with(cx, |scene, _| scene.volume), 0);
    cx.simulate_keystrokes("end right");
    assert_eq!(scene.read_with(cx, |scene, _| scene.volume), 100);
}

#[gpui::test]
fn dropdown_mouse_slider_drag_and_switch(cx: &mut TestAppContext) {
    let (scene, cx) = open(cx);
    click(cx, "priority");
    click(cx, "priority-high");
    click(cx, "sounds");
    let bounds = scene.read_with(cx, |scene, _| scene.slider_bounds.unwrap());
    cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Default::default());
    cx.simulate_mouse_move(
        point(bounds.right() + px(20.), bounds.center().y),
        MouseButton::Left,
        Default::default(),
    );
    cx.simulate_mouse_up(
        point(bounds.right() + px(20.), bounds.center().y),
        MouseButton::Left,
        Default::default(),
    );
    scene.read_with(cx, |scene, _| {
        assert_eq!(scene.priority, 2);
        assert!(scene.sounds);
        assert_eq!(scene.volume, 100);
        assert!(!scene.dragging_slider);
    });
}

#[gpui::test]
fn modal_traps_tab_cancels_by_default_and_restores_focus(cx: &mut TestAppContext) {
    let (scene, cx) = open(cx);
    click(cx, "open-modal");
    scene.update_in(cx, |scene, window, _| {
        assert!(scene.modal);
        assert!(scene.cancel_focus.is_focused(window));
    });
    cx.simulate_keystrokes("shift-tab");
    scene.update_in(cx, |scene, window, _| {
        assert!(scene.confirm_focus.is_focused(window))
    });
    cx.simulate_keystrokes("enter");
    assert!(scene.read_with(cx, |scene, _| scene.modal));
    assert_eq!(scene.read_with(cx, |scene, _| scene.confirmations), 0);
    cx.simulate_keystrokes("tab enter");
    scene.update_in(cx, |scene, window, _| {
        assert!(!scene.modal);
        assert_eq!(scene.confirmations, 0);
        assert!(scene.modal_focus.is_focused(window));
    });
    cx.simulate_keystrokes("enter escape enter tab space");
    scene.read_with(cx, |scene, _| {
        assert!(!scene.modal);
        assert_eq!(scene.confirmations, 1);
    });
}

#[gpui::test]
fn wheel_scrolls_rows(cx: &mut TestAppContext) {
    let (scene, cx) = open(cx);
    let bounds = cx.debug_bounds("rows").unwrap();
    cx.simulate_event(ScrollEvent {
        position: bounds.center(),
        delta: ScrollDelta::Lines(point(0., -5.)),
        ..Default::default()
    });
    assert!(scene.read_with(cx, |scene, _| scene.scroll.offset().y) < px(0.));
}
