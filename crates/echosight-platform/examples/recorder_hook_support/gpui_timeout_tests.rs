// Opt-in real GPUI dispatch/timer tests. No Win32 hook, text handler or recorder.
// Only the documented runner enables this cfg and GPUI's test-support feature.
use crate::canaries::{self, LongChordCanary, ShortPrefixCanary};
use gpui::{
    Context, Entity, FocusHandle, KeyBinding, TestAppContext, VisualTestContext, Window, div,
    prelude::*,
};
use std::time::Duration;

struct TimeoutView {
    focus: FocusHandle,
    short: usize,
    long: usize,
}
impl Render for TimeoutView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus)
            .key_context("S9")
            .on_action(cx.listener(|this, _: &ShortPrefixCanary, _, _| this.short += 1))
            .on_action(cx.listener(|this, _: &LongChordCanary, _, _| this.long += 1))
    }
}
fn open(
    cx: &mut TestAppContext,
    bindings: Vec<KeyBinding>,
) -> (Entity<TimeoutView>, &mut VisualTestContext) {
    cx.update(|cx| cx.bind_keys(bindings));
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        TimeoutView {
            focus,
            short: 0,
            long: 0,
        }
    });
    cx.run_until_parked();
    (view, cx)
}

#[gpui::test]
fn long_only_prefix_has_pending_input_but_no_timeout(cx: &mut TestAppContext) {
    let (view, cx) = open(
        cx,
        vec![KeyBinding::new(
            "ctrl-k ctrl-c",
            LongChordCanary,
            Some("S9"),
        )],
    );
    cx.simulate_keystrokes("ctrl-k");
    cx.update(|window, cx| {
        let pending = window.pending_input().expect("prefix must be pending");
        assert_eq!(pending.keystrokes().len(), 1);
        assert!(
            pending.timeout().is_none(),
            "old canary did not create a timeout"
        );
        assert!(!canaries::has_live_ctrl_k_timeout(window, cx));
    });
    cx.background_executor.advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |v, _| (v.short, v.long)), (0, 0));
}

#[gpui::test]
fn ambiguous_prefix_has_real_timeout_and_replays_short_action(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, canaries::bindings());
    cx.update(|window, cx| assert!(!canaries::has_live_ctrl_k_timeout(window, cx)));
    cx.simulate_keystrokes("ctrl-k");
    let duration = cx.update(|window, cx| {
        let pending = window.pending_input().expect("prefix must be pending");
        assert!(
            pending.timeout().is_some(),
            "pending keys alone are not a timeout test"
        );
        let timeout = pending.timeout().unwrap();
        assert!(!timeout.is_paused());
        assert!(canaries::has_live_ctrl_k_timeout(window, cx));
        timeout.duration()
    });
    assert_eq!(view.read_with(cx, |v, _| (v.short, v.long)), (0, 0));
    cx.background_executor
        .advance_clock(duration + Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |v, _| (v.short, v.long)), (1, 0));
    cx.update(|window, cx| {
        assert!(window.pending_input().is_none());
        assert!(!canaries::has_live_ctrl_k_timeout(window, cx));
    });
}

#[gpui::test]
fn completed_long_chord_cancels_short_timeout(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, canaries::bindings());
    cx.simulate_keystrokes("ctrl-k");
    let duration = cx.update(|window, cx| {
        assert!(canaries::has_live_ctrl_k_timeout(window, cx));
        window
            .pending_input()
            .unwrap()
            .timeout()
            .expect("real timer")
            .duration()
    });
    cx.simulate_keystrokes("ctrl-c");
    assert_eq!(view.read_with(cx, |v, _| (v.short, v.long)), (0, 1));
    cx.update(|window, cx| {
        assert!(window.pending_input().is_none());
        assert!(!canaries::has_live_ctrl_k_timeout(window, cx));
    });
    cx.background_executor
        .advance_clock(duration + Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |v, _| (v.short, v.long)), (0, 1));
}
